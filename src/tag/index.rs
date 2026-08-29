use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    index::{FileIndexer, IndexInterest},
    repository::io::ScopedPath,
    tag::{json::KnownTagsRegistry, sqlite::SqliteTagRepository},
    util::TryCollectN,
};

pub struct TagIndexer {
    pub json: Arc<KnownTagsRegistry>,
    pub sqlite: Arc<Mutex<SqliteTagRepository>>,
}

impl TagIndexer {
    pub fn new(json: Arc<KnownTagsRegistry>, sqlite: Arc<Mutex<SqliteTagRepository>>) -> Self {
        Self { json, sqlite }
    }
}

impl FileIndexer for TagIndexer {
    fn get_index_name(&self, path: &ScopedPath) -> anyhow::Result<Option<IndexInterest>> {
        if let Some(["_def", "tags.json"]) = path
            .use_relative_path()
            .components()
            .map(|str| str.as_os_str().to_str().unwrap())
            .try_collect_n()
        {
            Ok(Some(IndexInterest {
                name: "tags".to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    fn indexer_version(&self) -> &'static str {
        "tag-v0"
    }

    async fn index(&self, _path: &ScopedPath) -> anyhow::Result<()> {
        let known_tags = self.json.get().await?;

        let sqlite = self.sqlite.lock().await;
        sqlite.replace_with_known_tags_set(known_tags).await?;

        Ok(())
    }
}
