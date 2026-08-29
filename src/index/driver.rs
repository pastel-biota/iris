use std::fs::Metadata;

use anyhow::Context;

use crate::{
    index::{history::IndexHistory, indexer::Indexers},
    repository::io::ScopedPath,
};

pub struct FileIndexDriver {
    dir: ScopedPath,
    history: IndexHistory,
    indexers: Indexers,
}

impl FileIndexDriver {
    pub fn new(dir: ScopedPath, history: IndexHistory, indexers: Indexers) -> Self {
        FileIndexDriver {
            dir,
            history,
            indexers,
        }
    }

    pub async fn fully_reindex(&self) {
        tracing::info!("Starting to re-index");

        let mut re_indexed = 0usize;
        let mut skipped = 0usize;
        let mut errored = 0usize;

        for file in self.dir.walk_files() {
            let file = match file {
                Ok(file) => file,
                Err(err) => {
                    tracing::warn!("Error during scanning the JSON: {}", err);
                    errored += 1;
                    continue;
                }
            };

            match self.re_index_file(&file.path, file.metadata).await {
                Ok(true) => re_indexed += 1,
                Ok(false) => skipped += 1,
                Err(err) => {
                    let path = file.path.display().to_string();
                    tracing::warn!(path = path, "Error during processing the JSON: {}", err);
                    errored += 1
                }
            }
        }

        tracing::info!(re_indexed, skipped, errored, "Re-indexed JSON files");
    }

    pub async fn index_file(&self, path: &ScopedPath) -> anyhow::Result<()> {
        let metadata = path
            .use_path()
            .metadata()
            .context("while retrieving the processing file's metadata (file not existent?)")?;

        let reindexed = self.re_index_file(path, metadata).await?;

        if reindexed {
            tracing::debug!("The file has been reindexed");
        } else {
            tracing::debug!("The file was not reindexed (likely irrevalent file)");
        }

        Ok(())
    }

    async fn re_index_file(&self, path: &ScopedPath, metadata: Metadata) -> anyhow::Result<bool> {
        let displayed_path = path.display().to_string();

        let Some((indexer, interest)) = self.indexers.find_interested_indexer(path)? else {
            tracing::trace!(path = displayed_path, "No indexer is inteersted");
            return Ok(false);
        };

        let need_reindex = self
            .history
            .need_reindex(indexer.indexer_version(), &interest.name, path, &metadata)
            .await;

        let need_reindex = match need_reindex {
            Ok(need_reindex) => need_reindex,
            Err(err) => anyhow::bail!(
                "Error during checking the JSON file needs re-indexing: {}: {}",
                &interest.name,
                err
            ),
        };

        if !need_reindex {
            tracing::trace!(path = displayed_path, "This file doesn't need new indexing");
            return Ok(false);
        }

        indexer.index(path).await?;

        self.history
            .record_index(indexer.indexer_version(), &interest.name, path, &metadata)
            .await
            .unwrap();

        Ok(true)
    }
}
