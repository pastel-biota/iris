use crate::{
    config::Config,
    services::property::{ProcessorContext, create_property_processor_context},
};

pub mod process;
pub mod property;
pub mod resize;

pub struct ServiceContext {
    pub proceessor: ProcessorContext,
}

pub fn build_service_context(config: &Config) -> Result<ServiceContext, anyhow::Error> {
    Ok(ServiceContext {
        proceessor: create_property_processor_context(&config.processors)?,
    })
}
