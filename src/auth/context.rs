use std::{collections::HashMap, sync::Mutex};

use crate::auth::{config::AuthConfig, session::SessionsStore};

pub struct AuthContext {
    pub config: AuthConfig,
    pub state: Mutex<RuntimeState>
}

#[derive(Default)]
pub struct RuntimeState {
    pub sessions: SessionsStore,
}

impl AuthContext {
    pub fn new(config: AuthConfig) -> Self {
        AuthContext {
            config,
            state: Mutex::new(Default::default()),
        }
    }
}


