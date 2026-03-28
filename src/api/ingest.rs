use config::IngestConfig;

pub mod config;
pub mod route;
pub mod technicals;

pub struct IngestContext {
    pub config: IngestConfig,
}

impl IngestContext {
    pub fn new(config: IngestConfig) -> Self {
        Self { config }
    }
}

