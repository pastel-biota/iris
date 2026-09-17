use std::sync::Arc;

use tokio::sync::Mutex;

use crate::whitelist::sqlite::SqliteWhitelistRepository;

pub struct WhitelistContext {
    pub repo: Arc<Mutex<SqliteWhitelistRepository>>,
}

impl WhitelistContext {
    pub fn new(repo: Arc<Mutex<SqliteWhitelistRepository>>) -> Self {
        WhitelistContext { repo }
    }
}
