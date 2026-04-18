use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::auth::password::HashedPassword;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    pub entities: HashMap<String, Entity>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Entity {
    User(UserEntity),
    Federation(FederationEntity),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserEntity {
    pub password: HashedPassword,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationEntity {
}

pub fn serialize_new_user(name: &str, password: HashedPassword) -> anyhow::Result<String> {
    let mut config = AuthConfig::default();

    config.entities.insert(
        name.to_string(),
        Entity::User(UserEntity { password })
    );

    Ok(toml::to_string(&config)?)
}

