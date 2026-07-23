#![deny(clippy::disallowed_types)]

use std::{process::ExitCode, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    auth::context::AuthContext, config::{BaseConfig, Entry, parse_config}, entry::server::RunServerResourcees, event::{EventSender, create_event_bus}, infra::{api::rate_limit::RateLimit, sqlite::SqliteConnection}, ingest::context::{IngestContext, ServiceContext}, processor::ProcessorContext, repository::{io::ScopedPath, registry::PhotoStorageRegistry},
};

pub mod config;
pub mod entry;
pub mod event;
pub mod infra;
pub mod ingest;
pub mod model;
pub mod processor;
pub mod repository;
pub mod services;
pub mod util;
pub mod auth;
pub mod federation;
pub mod api;
pub mod macros;

pub struct Context {
    pub base: BaseConfig,
    pub auth: AuthContext,
    pub ingest: IngestContext,
    pub registry: RwLock<PhotoStorageRegistry>,
    pub service: ServiceContext,
    pub processor: ProcessorContext,
    pub event_tx: EventSender,
    pub rate_limit: RateLimit,
}

#[tokio::main]
async fn main() -> ExitCode {
    crate::infra::log::initialize_log();

    match run().await {
        Ok(_) => {
            eprintln!("Iris is exiting");
            0.into()
        }
        Err(e) => {
            eprintln!("The Iris experienced fatal issue and cannot continue:");
            eprintln!("{e}");
            tracing::error!("The Iris is exiting: {e}");

            for cause in e.chain().skip(1) {
                eprintln!("  <- {cause}");
                tracing::error!("<- {cause}");
            }

            1.into()
        }
    }
}

async fn run() -> Result<(), anyhow::Error> {
    let Entry { command, config } = parse_config()?;
    let (event_tx, event_rx) = create_event_bus(64);

    let ingest_scope = ScopedPath::from_allowed_dir(&config.ingest.dir);

    let sqlite = SqliteConnection::connect(&ingest_scope).await?;

    let ctx = Arc::new(Context {
        registry: RwLock::new(PhotoStorageRegistry::new(config.federation, sqlite.clone(), &ingest_scope)),
        service: ServiceContext::try_from_config(&config.processors)?,
        auth: AuthContext::new(config.auth, sqlite.clone()),
        ingest: IngestContext::new(config.ingest),
        processor: ProcessorContext::new(config.image),
        base: config.base,
        event_tx,
        rate_limit: RateLimit::default(),
    });

    match command {
        config::Command::Server => {
            let resources = RunServerResourcees { ctx, event_rx };
            entry::server::run_server(resources).await?;
        },
        config::Command::User(user_config) => {
            entry::user::create_user(ctx, user_config).await?;
        },
        config::Command::Migration(migration_config) => {
            entry::migrate::migrate(ctx, ingest_scope, migration_config).await?;
        },
    } 

    Ok(())
}
