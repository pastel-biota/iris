use crate::{config::Config, services::property::{ProcessorContext, create_property_processor_context}};

pub mod property;

pub struct ServiceContext {
    pub proceessor: ProcessorContext,
}

pub fn build_service_context(config: &Config) -> Result<ServiceContext, anyhow::Error> {
    Ok(ServiceContext {
        proceessor: create_property_processor_context(&config.processors)?,
    })
}

