use std::collections::HashMap;

use anyhow::Context;
use base64::{Engine, prelude::BASE64_URL_SAFE};
use rand::RngExt;

use crate::auth::config::Entity;

pub const SESSION_DURATION: chrono::Duration = chrono::Duration::days(7);

#[derive(Default, Debug)]
pub struct SessionsStore(HashMap<String, Session>);

#[derive(Clone, Debug)]
pub struct Session {
    pub entity: Entity,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl SessionsStore {
    pub fn new_session(&mut self, entity: Entity) -> anyhow::Result<String> {
        let session_id: [u8; 64] = rand::make_rng::<rand::rngs::StdRng>().random();
        let session_key = BASE64_URL_SAFE.encode(session_id);
    
        self.0.insert(session_key.clone(), Session::issue_new(entity));
    
        Ok(session_key)
    }

    pub fn get_session(&mut self, key: &str) -> anyhow::Result<Option<Session>> {
        let Some(session) = self.0.get(key).cloned() else {
            return Ok(None);
        };

        if session.expires_at <= chrono::Utc::now() {
            self.0.remove(key);
            return Ok(None);
        }

        Ok(Some(session))
    }
}

impl Session {
    pub fn issue_new(entity: Entity) -> Self {
        Self {
            entity,
            expires_at: chrono::Utc::now() + SESSION_DURATION,
        }
    }
}

