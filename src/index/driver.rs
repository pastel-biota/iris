use std::fs::Metadata;
use std::sync::Arc;

use anyhow::Context;
use tokio::sync::RwLock;

use super::photo::PhotoIndexer;
use crate::{
    index::{FileIndexer, history::IndexHistory},
    repository::{io::ScopedPath, registry::PhotoStorageRegistry},
};

pub struct FileIndexDriver {
    dir: ScopedPath,
    history: IndexHistory,
    photo_indexer: PhotoIndexer,
}

impl FileIndexDriver {
    pub fn new(
        dir: ScopedPath,
        history: IndexHistory,
        registry: Arc<RwLock<PhotoStorageRegistry>>,
    ) -> Self {
        FileIndexDriver {
            dir,
            history,
            photo_indexer: PhotoIndexer::new(registry),
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

        // TODO: Maybe replace with Vec<Box<dyn FileIndexer>>
        let Some(interest) = self.photo_indexer.get_index_name(&path)? else {
            tracing::trace!(path = displayed_path, "No indexer is inteersted");
            return Ok(false);
        };

        let need_reindex = self
            .history
            .need_reindex(
                self.photo_indexer.indexer_version(),
                &interest.name,
                path,
                &metadata,
            )
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

        self.photo_indexer.index(interest.interest, path).await?;

        self.history
            .record_index(
                self.photo_indexer.indexer_version(),
                &interest.name,
                path,
                &metadata,
            )
            .await
            .unwrap();

        Ok(true)
    }
}
