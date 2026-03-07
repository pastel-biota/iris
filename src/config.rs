use serde::Deserialize;

use crate::services::property::ProcessorConfig;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub processors: ProcessorConfig,
}

pub fn parse_config() -> anyhow::Result<Config> {
    let file = std::fs::read_to_string("./iris.toml")?;
    Ok(toml::from_str(&file)?)
}
