use crate::config::Config;

use self::property::{ProcessorContext, create_property_processor_context};

pub mod process;
pub mod property;

pub struct ServiceContext {
    pub proceessor: ProcessorContext,
}

impl ServiceContext {
    pub fn try_from_config(config: &Config) -> Result<ServiceContext, anyhow::Error> {
        Ok(Self {
            proceessor: create_property_processor_context(&config.processors)?,
        })
    }
}
