use std::path::Path;

use crate::federation::{config::FederationConfig, repository::FederationRepository};

pub struct FederationContext {
    pub config: FederationConfig,
    pub repo: FederationRepository,
}

impl FederationContext {
    pub fn new(global_dir: &Path, config: FederationConfig) -> Self {
        Self {
            repo: FederationRepository::new(global_dir),
            config,
        }
    }
}
