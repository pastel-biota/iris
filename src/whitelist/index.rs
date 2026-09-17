use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    index::{FileIndexer, IndexInterest},
    repository::io::ScopedPath,
    util::TryCollectN,
    whitelist::{json::WhitelistRegistry, sqlite::SqliteWhitelistRepository},
};

pub struct WhitelistIndexer {
    pub json: Arc<WhitelistRegistry>,
    pub sqlite: Arc<Mutex<SqliteWhitelistRepository>>,
}

impl WhitelistIndexer {
    pub fn new(
        json: Arc<WhitelistRegistry>,
        sqlite: Arc<Mutex<SqliteWhitelistRepository>>,
    ) -> Self {
        Self { json, sqlite }
    }
}

impl FileIndexer for WhitelistIndexer {
    fn get_index_name(&self, path: &ScopedPath) -> anyhow::Result<Option<IndexInterest>> {
        if let Some(["_def", "whitelist.json"]) = path
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
        "whitelist-v0"
    }

    async fn index(&self, _path: &ScopedPath) -> anyhow::Result<()> {
        let known_tags = self.json.get().await?;

        let sqlite = self.sqlite.lock().await;
        sqlite.update_whitelist(known_tags).await?;

        Ok(())
    }
}
