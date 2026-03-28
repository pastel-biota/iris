use crate::repository::FederationConfig;

pub mod route;

pub struct FederationContext {
    pub config: FederationConfig,
}

impl FederationContext {
    pub fn new(config: FederationConfig) -> Self {
        Self { config }
    }
}


