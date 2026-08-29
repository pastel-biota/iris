use std::sync::Arc;

use tokio::sync::Mutex;

use crate::tag::sqlite::SqliteTagRepository;

pub struct TagContext {
    pub repo: Arc<Mutex<SqliteTagRepository>>,
}

impl TagContext {
    pub fn new(repo: Arc<Mutex<SqliteTagRepository>>) -> Self {
        TagContext { repo }
    }
}
