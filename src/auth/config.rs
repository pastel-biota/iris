use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{auth::password::HashedPassword, model::EntityName};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    pub entities: HashMap<EntityName, Entity>,
    pub unrestricted_instance: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Entity {
    User(UserEntity),
    Federation(FederationEntity),
}

impl Entity {
    pub fn name(&self) -> &EntityName {
        match self {
            Entity::User(entity) => &entity.name,
            Entity::Federation(entity) => &entity.name,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserEntity {
    pub name: EntityName,
    pub password: HashedPassword,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FederationEntity {
    pub name: EntityName,
    pub password: HashedPassword,
}

pub fn serialize_new_user(name: EntityName, password: HashedPassword) -> anyhow::Result<String> {
    let mut config = AuthConfig::default();

    config.entities.insert(
        name.clone(),
        Entity::User(UserEntity { name, password })
    );

    Ok(toml::to_string(&config)?)
}

