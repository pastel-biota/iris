pub mod driver;
pub mod history;
pub mod photo;
pub mod watch;

use std::sync::Arc;

use diesel::result::DatabaseErrorInformation;
use tokio::sync::RwLock;

use crate::{
    index::{driver::FileIndexDriver, history::IndexHistory},
    infra::sqlite::SqliteConnection,
    repository::{io::ScopedPath, registry::PhotoStorageRegistry},
};

pub struct IndexInterest<T> {
    name: String,
    interest: T,
}

pub trait FileIndexer {
    type PathInterest;

    fn get_index_name(
        &self,
        path: &ScopedPath,
    ) -> anyhow::Result<Option<IndexInterest<Self::PathInterest>>>;
    fn indexer_version(&self) -> &'static str;
    fn index(
        &self,
        path: Self::PathInterest,
        file: &ScopedPath,
    ) -> impl Future<Output = anyhow::Result<()>>;
}

pub struct IndexerContext {
    driver: FileIndexDriver,
}

impl IndexerContext {
    pub fn new(
        dir: &ScopedPath,
        db: SqliteConnection,
        registry: Arc<RwLock<PhotoStorageRegistry>>,
    ) -> Self {
        Self {
            driver: FileIndexDriver::new(dir.clone(), IndexHistory::new(db), registry),
        }
    }
}
