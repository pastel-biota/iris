use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    model::{EntityName, Whitelist},
    repository::io::ScopedPath,
};

pub struct WhitelistRegistry {
    scope: ScopedPath,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
pub enum WhitelistRegistryPayload {
    V1(v1::WhitelistRegistryPayload),
}

pub type WhitelistConfiguration = HashMap<EntityName, Whitelist>;

mod v1 {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    use crate::model::{EntityName, Whitelist};

    #[derive(Clone, Serialize, Deserialize)]
    pub struct WhitelistRegistryPayload {
        entities: HashMap<EntityName, Whitelist>,
    }

    impl From<WhitelistRegistryPayload> for super::WhitelistConfiguration {
        fn from(value: WhitelistRegistryPayload) -> Self {
            value.entities
        }
    }
}

impl WhitelistRegistry {
    pub fn new(scope: ScopedPath) -> Self {
        Self { scope }
    }

    pub async fn get(&self) -> anyhow::Result<WhitelistConfiguration> {
        let path = self.scope.join("_def/whitelist.json");

        let WhitelistRegistryPayload::V1(payload) =
            serde_json::from_slice(path.read_binary()?.as_slice())?;

        Ok(payload.into())
    }
}
