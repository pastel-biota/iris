use crate::config::Config;
use crate::ingest::config::IngestConfig;
use crate::ingest::technicals::image::property::{PropertyContext, create_property_processor_context};

pub struct IngestContext {
    pub config: IngestConfig,
}

impl IngestContext {
    pub fn new(config: IngestConfig) -> Self {
        Self { config }
    }
}

pub struct ServiceContext {
    pub property: PropertyContext,
}

impl ServiceContext {
    pub fn try_from_config(config: &Config) -> Result<ServiceContext, anyhow::Error> {
        Ok(Self {
            property: create_property_processor_context(&config.processors)?,
        })
    }
}
