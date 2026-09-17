#![deny(clippy::disallowed_types)]

use std::{process::ExitCode, sync::Arc};

use tokio::sync::{Mutex, RwLock};

use crate::{
    auth::context::AuthContext,
    config::{BaseConfig, Entry, parse_config},
    entry::server::RunServerResourcees,
    event::{EventSender, create_event_bus},
    index::{IndexerContext, indexer::Indexers, photo::PhotoIndexer},
    infra::{api::rate_limit::RateLimit, sqlite::SqliteConnection},
    ingest::context::{IngestContext, ServiceContext},
    processor::ProcessorContext,
    repository::{io::ScopedPath, registry::PhotoStorageRegistry},
    tag::{
        context::TagContext, index::TagIndexer, json::KnownTagsRegistry,
        sqlite::SqliteTagRepository,
    },
    whitelist::{
        index::WhitelistIndexer, json::WhitelistRegistry, sqlite::SqliteWhitelistRepository,
    },
};

pub mod api;
pub mod auth;
pub mod config;
pub mod entry;
pub mod event;
pub mod federation;
pub mod index;
pub mod infra;
pub mod ingest;
pub mod macros;
pub mod model;
pub mod processor;
pub mod repository;
pub mod services;
pub mod tag;
pub mod util;
pub mod whitelist;

pub struct Context {
    pub scope: ScopedPath,
    pub base: BaseConfig,
    pub auth: AuthContext,
    pub ingest: IngestContext,
    pub tag: TagContext,
    pub registry: Arc<RwLock<PhotoStorageRegistry>>,
    pub service: ServiceContext,
    pub processor: ProcessorContext,
    pub event_tx: EventSender,
    pub rate_limit: RateLimit,
    pub indexer: IndexerContext,
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

    let registry = Arc::new(RwLock::new(PhotoStorageRegistry::new(
        config.federation,
        sqlite.clone(),
        &ingest_scope,
    )));

    let tag_registry = Arc::new(Mutex::new(SqliteTagRepository::new(sqlite.clone())));

    let indexer = Indexers {
        photo_indexer: PhotoIndexer {
            registry: registry.clone(),
        },
        tag_indexer: TagIndexer {
            json: Arc::new(KnownTagsRegistry::new(ingest_scope.clone())),
            sqlite: tag_registry.clone(),
        },
        whitelist_indexer: WhitelistIndexer {
            json: Arc::new(WhitelistRegistry::new(ingest_scope.clone())),
            sqlite: Arc::new(Mutex::new(SqliteWhitelistRepository::new(sqlite.clone()))),
        },
    };

    let ctx = Arc::new(Context {
        scope: ingest_scope.clone(),
        registry: registry.clone(),
        service: ServiceContext::try_from_config(&config.processors)?,
        auth: AuthContext::new(config.auth, sqlite.clone()),
        tag: TagContext::new(tag_registry.clone()),
        ingest: IngestContext::new(config.ingest),
        processor: ProcessorContext::new(config.image),
        base: config.base,
        event_tx,
        rate_limit: RateLimit::default(),
        indexer: IndexerContext::new(&ingest_scope, sqlite.clone(), indexer),
    });

    match command {
        config::Command::Server => {
            let resources = RunServerResourcees { ctx, event_rx };
            entry::server::run_server(resources).await?;
        }
        config::Command::User(user_config) => {
            entry::user::create_user(ctx, user_config).await?;
        }
        config::Command::Migration(migration_config) => {
            entry::migrate::migrate(ctx, ingest_scope, migration_config).await?;
        }
    }

    Ok(())
}
