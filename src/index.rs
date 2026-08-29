pub mod driver;
pub mod history;
pub mod indexer;
pub mod photo;
pub mod watch;

use crate::{
    index::{driver::FileIndexDriver, history::IndexHistory, indexer::Indexers},
    infra::sqlite::SqliteConnection,
    repository::io::ScopedPath,
};

pub struct IndexInterest {
    pub name: String,
}

pub trait FileIndexer {
    fn get_index_name(&self, path: &ScopedPath) -> anyhow::Result<Option<IndexInterest>>;
    fn indexer_version(&self) -> &'static str;
    fn index(&self, file: &ScopedPath) -> impl Future<Output = anyhow::Result<()>>;
}

pub struct IndexerContext {
    driver: FileIndexDriver,
}

impl IndexerContext {
    pub fn new(dir: &ScopedPath, db: SqliteConnection, indexers: Indexers) -> Self {
        Self {
            driver: FileIndexDriver::new(dir.clone(), IndexHistory::new(db), indexers),
        }
    }
}
