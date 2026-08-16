use std::collections::HashMap;

use base64::{Engine, prelude::BASE64_URL_SAFE};
use rand::RngExt;

use crate::{auth::config::Entity, model::PhotoVisibility};

pub const SESSION_DURATION: chrono::Duration = chrono::Duration::days(7);

#[derive(Default, Debug)]
pub struct SessionsStore(HashMap<String, ValidSession>);

#[derive(Clone, Debug)]
pub enum Session {
    Valid(ValidSession),
    Bypassed,
}

impl Session {
    pub fn bypass_or_verify(&self, verify_fn: impl FnOnce(&ValidSession) -> bool) -> bool {
        match self {
            Session::Valid(valid_session) => verify_fn(valid_session),
            Session::Bypassed => true,
        }
    }

    pub fn bypass_or_ensure<E>(
        &self,
        ensure_fn: impl FnOnce(&ValidSession) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Session::Valid(valid_session) => ensure_fn(valid_session),
            Session::Bypassed => Ok(()),
        }
    }

    pub fn not_bypassed(&self) -> Option<&ValidSession> {
        match self {
            Session::Valid(valid_session) => Some(valid_session),
            Session::Bypassed => None,
        }
    }

    pub fn is_bypassed(&self) -> bool {
        match self {
            Session::Valid(_) => false,
            Session::Bypassed => true,
        }
    }

    pub fn photo_visibility(&self) -> PhotoVisibility {
        match self {
            Session::Bypassed => PhotoVisibility::All,
            Session::Valid(session) if session.entity.has_full_image_access() => {
                PhotoVisibility::All
            }
            Session::Valid(session) => {
                PhotoVisibility::WhitelistedFor(session.entity.name().clone())
            }
        }
    }
}

impl From<ValidSession> for Session {
    fn from(value: ValidSession) -> Self {
        Session::Valid(value)
    }
}

#[derive(Clone, Debug)]
pub struct ValidSession {
    pub entity: Entity,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl SessionsStore {
    pub fn new_session(&mut self, entity: Entity) -> anyhow::Result<String> {
        let session_id: [u8; 64] = rand::make_rng::<rand::rngs::StdRng>().random();
        let session_key = BASE64_URL_SAFE.encode(session_id);

        self.0
            .insert(session_key.clone(), ValidSession::issue_new(entity));

        Ok(session_key)
    }

    pub fn get_session(&mut self, key: &str) -> anyhow::Result<Option<ValidSession>> {
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

impl ValidSession {
    pub fn issue_new(entity: Entity) -> Self {
        Self {
            entity,
            expires_at: chrono::Utc::now() + SESSION_DURATION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityName;
    use std::str::FromStr as _;

    #[test]
    fn bypassed_session_can_see_all_photos() {
        assert_eq!(Session::Bypassed.photo_visibility(), PhotoVisibility::All);
    }

    #[test]
    fn full_access_entity_can_see_all_photos_without_being_bypassed() {
        let session = Session::Valid(ValidSession::issue_new(entity("full", true)));

        assert!(!session.is_bypassed());
        assert_eq!(session.photo_visibility(), PhotoVisibility::All);
    }

    #[test]
    fn restricted_entity_can_only_see_its_whitelisted_photos() {
        let session = Session::Valid(ValidSession::issue_new(entity("restricted", false)));

        assert_eq!(
            session.photo_visibility(),
            PhotoVisibility::WhitelistedFor(EntityName::from_str("restricted").unwrap())
        );
    }

    fn entity(name: &str, has_full_image_access: bool) -> Entity {
        serde_json::from_value(serde_json::json!({
            "user": {
                "name": name,
                "password": "unused-in-this-test",
                "has_full_image_access": has_full_image_access,
            }
        }))
        .unwrap()
    }
}
