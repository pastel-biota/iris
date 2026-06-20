use crate::ingest::config::IngestConfig;
use crate::ingest::technicals::image::property::{PropertyConfig, PropertyContext, create_property_processor_context};
use crate::ingest::technicals::stream::SizedStream;

pub struct IngestContext {
    pub config: IngestConfig,
    pub sized_sream: SizedStream,
}

impl IngestContext {
    pub fn new(config: IngestConfig) -> Self {
        Self {
            config,
            sized_sream: SizedStream::new(),
        }
    }
}

pub struct ServiceContext {
    pub property: PropertyContext,
}

impl ServiceContext {
    pub fn try_from_config(config: &PropertyConfig) -> Result<ServiceContext, anyhow::Error> {
        Ok(Self {
            property: create_property_processor_context(config)?,
        })
    }
}
