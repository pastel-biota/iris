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
    fn get_index_name(&self, path: &ScopedPath) -> anyhow::Result<Option<IndexInterest>> {
        let Some(id) = parse_path(path) else {
            return Ok(None);
        };

        Ok(Some(IndexInterest {
            name: format!("photo-{}", id),
        }))
    }

    fn indexer_version(&self) -> &'static str {
        "photo-v2_photo-tag"
    }

    async fn index(&self, path: &ScopedPath) -> anyhow::Result<()> {
        let id = parse_path(path).expect("The path should have been validated");

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

fn parse_path(path: &ScopedPath) -> Option<LocalIdentifier> {
    let components = path.use_relative_path().components();

    let Some([year, month, json]) = components
        .map(|comp| {
            comp.as_os_str()
                .to_str()
                .expect("The path is not UTF-8 compatible")
        })
        .try_collect_n()
    else {
        return None;
    };

    if !year.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }

    if !month.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }

    let Some((file_name, "json")) = json.split_once(".") else {
        return None;
    };

    let Some((id, "meta")) = file_name.rsplit_once("-") else {
        return None;
    };

    let id = id.parse().ok()?;
    Some(LocalIdentifier(id))
}
