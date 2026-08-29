use std::sync::Arc;

use anyhow::bail;
use tokio::sync::RwLock;

use crate::{
    index::{FileIndexer, IndexInterest},
    model::LocalIdentifier,
    repository::{io::ScopedPath, registry::PhotoStorageRegistry},
    util::TryCollectN,
};

pub struct PhotoIndexer {
    pub registry: Arc<RwLock<PhotoStorageRegistry>>,
}

impl PhotoIndexer {
    pub fn new(registry: Arc<RwLock<PhotoStorageRegistry>>) -> Self {
        Self { registry }
    }
}

impl FileIndexer for PhotoIndexer {
    type PathInterest = LocalIdentifier;

    fn get_index_name(
        &self,
        path: &ScopedPath,
    ) -> anyhow::Result<Option<IndexInterest<Self::PathInterest>>> {
        let components = path.use_relative_path().components();

        let Some([year, month, json]) = components
            .map(|comp| {
                comp.as_os_str()
                    .to_str()
                    .expect("The path is not UTF-8 compatible")
            })
            .try_collect_n()
        else {
            return Ok(None);
        };

        if !year.chars().all(|char| char.is_ascii_digit()) {
            return Ok(None);
        }

        if !month.chars().all(|char| char.is_ascii_digit()) {
            return Ok(None);
        }

        let Some((file_name, "json")) = json.split_once(".") else {
            return Ok(None);
        };

        let Some((id, "meta")) = file_name.rsplit_once("-") else {
            return Ok(None);
        };

        let Ok(id) = id.parse() else {
            return Ok(None);
        };

        Ok(Some(IndexInterest {
            name: format!("photo-{}", id),
            interest: LocalIdentifier(id),
        }))
    }

    fn indexer_version(&self) -> &'static str {
        "photo-v2_photo-tag"
    }

    async fn index(&self, id: LocalIdentifier, _file: &ScopedPath) -> anyhow::Result<()> {
        let mut registry = self.registry.write().await;

        let photo_meta = match registry.dir.load_photo_meta(&id) {
            Ok(photo_meta) => photo_meta,
            Err(err) => {
                bail!("Error during reading the PhotoMeta: {}: {}", id, err);
            }
        };

        registry.local_index.upsert_photo(photo_meta).await.unwrap();

        Ok(())
    }
}
