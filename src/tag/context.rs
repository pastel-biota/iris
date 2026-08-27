use std::sync::Arc;

use crate::{infra::sqlite::SqliteConnection, tag::sqlite::SqliteTagRepository};

pub struct TagContext {
    pub repo: Arc<SqliteTagRepository>,
}

impl TagContext {
    pub fn new(db: SqliteConnection) -> Self {
        TagContext {
            repo: Arc::new(SqliteTagRepository::new(db)),
        }
    }
}
