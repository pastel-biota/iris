use crate::config::Config;

use self::property::{PropertyContext, create_property_processor_context};

pub mod process;
pub mod property;

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
