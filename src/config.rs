use anyhow::Context;
use clap::Parser;
use config::Config as ConfigLoad;
use serde::Deserialize;

use crate::{
    ingest::config::IngestConfig,
    ingest::technicals::image::property::PropertyConfig,
    processor::config::ImageProcessConfig,
    federation::config::FederationConfig,
};

#[derive(clap::Parser)]
struct Args {
    #[clap(subcommand)]
    command: SubCommands,

    #[clap(flatten)]
    config: CommonOptions,
}

#[derive(Clone, clap::Subcommand)]
enum SubCommands {
    Server {
        #[clap(flatten)]
        config: CommonOptions,
    },
    User {
        #[clap(flatten)]
        config: CommonOptions,

        #[clap(flatten)]
        user: UserConfig,
    },
}

#[derive(Clone, Debug, clap::Args)]
pub struct UserConfig {
    #[clap(short, long)]
    pub name: String,
}

#[derive(Clone, clap::Args)]
struct CommonOptions {
    #[clap(short, long)]
    configs: Option<Vec<String>>,
}

impl CommonOptions {
    pub fn merge(self, other: Self) -> Self {
        let configs = match (self.configs, other.configs) {
            (Some(mut left), Some(right)) => {
                left.extend(right);
                Some(left)
            },
            (Some(one), None) | (None, Some(one)) => Some(one),
            (None, None) => None,
        };

        Self { configs }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseConfig,
    pub ingest: IngestConfig,
    pub processors: PropertyConfig,
    pub image: ImageProcessConfig,
    pub federation: FederationConfig,
}

#[derive(Debug, Deserialize)]
pub struct BaseConfig {
    pub host: String,
}

#[derive(Debug)]
pub struct Entry {
    pub config: Config,
    pub command: Command,
}

#[derive(Clone, Debug)]
pub enum Command {
    Server,
    User(UserConfig),
} 

pub fn parse_config() -> anyhow::Result<Entry> { let args = Args::parse();
    let (command, config) = match args.command {
        SubCommands::Server { config } => {
            (Command::Server, config)
        },
        SubCommands::User { config, user } => {
            (Command::User(user), config)
        }
    };

    let mut builder = ConfigLoad::builder();

    if let Some(configs) = args.config.merge(config).configs {
        for config in configs {
            builder = builder.add_source(config::File::with_name(&config));
        }
    } else {
        builder = builder.add_source(config::File::with_name("iris.toml"))
    }

    let config = builder
        .add_source(config::Environment::with_prefix("IRIS_CONFIG"))
        .build()
        .context("The config could not be read")?
        .try_deserialize()
        .context(
            "The config was found, but could not parse as the TOML, or the valid config object",
        )?;

    Ok(Entry { config, command })
}
