use std::collections::HashMap;

use crate::model::EntityName;

#[derive(Debug, serde::Deserialize, Default)]
#[serde(default)]
pub struct FederationConfig {
    pub hosts: HashMap<EntityName, FederationHost>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct FederationHost {
    // TODO: Replace with more better one, like asymm auth
    pub username: EntityName,
    pub password: String,
    pub origin: String,
}

