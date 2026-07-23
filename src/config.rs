// This module is a boundary between CLI / TOML and the rest of the app.
// Raw `PathBuf` is needed here for argument parsing and deserialization;
// paths cross into `ScopedPath` at the point they are used for I/O.
#![allow(clippy::disallowed_types)]

use std::{collections::{HashSet, VecDeque}, path::{Path, PathBuf}};

use anyhow::Context;
use clap::Parser;
use serde::Deserialize;

use crate::{
    auth::config::AuthConfig, entry::{migrate::MigrationOptions, user::UserOptions}, ingest::{config::IngestConfig, technicals::image::property::PropertyConfig}, model::EntityName, processor::config::ImageProcessConfig, repository::config::FederationConfig
};

#[cfg(feature = "federation")]
use crate::federation::config::FederationConfig;

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
        user: UserOptions,
    },
    Migration {
        #[clap(flatten)]
        config: CommonOptions,

        #[clap(subcommand)]
        migration: MigrationOptions,
    },
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
    #[serde(default)]
    pub auth: AuthConfig,
    pub ingest: IngestConfig,
    pub processors: PropertyConfig,
    pub image: ImageProcessConfig,
    #[serde(default)]
    pub federation: FederationConfig,
}

#[derive(Debug, Deserialize)]
pub struct BaseConfig {
    pub host: String,

    /// Disables all rate limiting on this instance when set to `false`.
    /// Intended for a private, trusted-network instance (e.g. behind Tailscale) where
    /// bulk uploads would otherwise trip the limits meant for a publicly exposed instance.
    pub rate_limit: Option<bool>,
}

#[derive(Debug)]
pub struct Entry {
    pub config: Config,
    pub command: Command,
}

#[derive(Clone, Debug)]
pub enum Command {
    Server,
    User(UserOptions),
    Migration(MigrationOptions),
} 

pub fn parse_config() -> anyhow::Result<Entry> {
    let args = Args::parse();
    let (command, config) = match args.command {
        SubCommands::Server { config } => {
            (Command::Server, config)
        },
        SubCommands::User { config, user } => {
            (Command::User(user), config)
        },
        SubCommands::Migration { config, migration } => {
            (Command::Migration(migration), config)
        }
    };


    let mut importing: VecDeque<PathBuf> = VecDeque::new();
    let mut known_files: HashSet<PathBuf> = HashSet::new();

    if let Some(configs) = args.config.merge(config).configs {
        importing.extend(configs.iter().map(|x| PathBuf::from(x)));
    } else {
        importing.push_back(PathBuf::from("iris.toml"));
    }

    tracing::trace!("Initial: {:?}", importing);

    let mut loaded_config: Option<config::Config> = None;
    while let Some(file_name) = importing.pop_front() {
        tracing::trace!("Popping the {}", file_name.display());
        // tracing::trace!("Left: {:?}", importing);


        let mut builder = config::Config::builder()
            .add_source(config::File::with_name(file_name.as_os_str().to_str().unwrap()));

        if let Some(loaded_config) = loaded_config.take() {
            builder = builder
                .add_source(loaded_config);
        }

        let config = builder.build()
            .with_context(|| format!("Error during parsing {}", file_name.display()))?;

        if let Ok(import) = config.get_array("import") {
            for import_value in import {
                let config::ValueKind::String(path) = &import_value.kind else {
                    anyhow::bail!("While reading {} - the import should be specified with the string", file_name.display());
                };

                let path = file_name.parent().unwrap_or(Path::new("")).join(path);

                if known_files.contains(&path) {
                    continue;
                }

                tracing::trace!("Pending import - {}", path.display());
                importing.push_back(path.clone());
                known_files.insert(path);
            }
        } else {
            tracing::trace!("There was no import or was invalid");
        }

        if importing.len() > 512 {
            panic!("Too much importing - isn't importing circulating??");
        }

        tracing::debug!("Loaded {}", file_name.display());

        loaded_config = Some(config);
    }

    let config = loaded_config
        .expect("At least one config should have been loaded, but nothing seems to be loaded")
        .try_deserialize()
        .context(
            "The config was found, but could not parse as the TOML, or the valid config object",
        )?;

    Ok(Entry { config, command })
}
