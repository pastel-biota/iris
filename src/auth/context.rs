use std::sync::Mutex;

use crate::{auth::{config::AuthConfig, session::SessionsStore}, repository::{io::ScopedPath, whitelist::WhitelistRepository}};

pub struct AuthContext {
    pub config: AuthConfig,
    pub state: Mutex<RuntimeState>
}

pub struct RuntimeState {
    pub sessions: SessionsStore,
    pub whitelist: WhitelistRepository,
}

impl AuthContext {
    pub fn new(config: AuthConfig, base_dir: &ScopedPath) -> Self {
        AuthContext {
            config,
            state: Mutex::new(RuntimeState {
                sessions: Default::default(),
                whitelist: WhitelistRepository::new(base_dir),
            }),
        }
    }
}

