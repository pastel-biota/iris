use anyhow::Context;
use clap::Parser;
use config::Config as ConfigLoad;
use serde::Deserialize;

use crate::{api::ingest::{config::IngestConfig, technicals::image::property::PropertyConfig}, processor::config::ImageProcessConfig};

#[derive(clap::Parser)]
pub struct Args {
    #[clap(short, long)]
    configs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ingest: IngestConfig,
    pub processors: PropertyConfig,
    pub image: ImageProcessConfig,
}

pub fn parse_config() -> anyhow::Result<Config> {
    let args = Args::parse();

    let mut builder = ConfigLoad::builder();

    if let Some(configs) = args.configs {
        for config in configs {
            builder = builder.add_source(config::File::with_name(&config));
        }
    } else {
        builder = builder.add_source(config::File::with_name("iris.toml"))
    }

    builder
        .add_source(config::Environment::with_prefix("IRIS_CONFIG"))
        .build()
        .context("The config could not be read")?
        .try_deserialize()
        .context(
            "The config was found, but could not parse as the TOML, or the valid config object",
        )
}
