use std::{process::ExitCode, sync::Arc};

use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

use crate::{
    config::{BaseConfig, parse_config}, event::{EventSender, create_event_bus}, federation::context::FederationContext, ingest::context::{IngestContext, ServiceContext}, processor::ProcessorContext, repository::registry::PhotoStorageRegistry
};

pub mod config;
pub mod event;
pub mod federation;
pub mod infra;
pub mod ingest;
pub mod model;
pub mod processor;
pub mod repository;
pub mod services;
pub mod util;

pub struct Context {
    pub base: BaseConfig,
    pub ingest: IngestContext,
    pub registry: RwLock<PhotoStorageRegistry>,
    pub service: ServiceContext,
    pub processor: ProcessorContext,
    pub event_tx: EventSender,
    pub federation: FederationContext,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

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
    let config = parse_config()?;
    let (event_tx, event_rx) = create_event_bus(64);

    let ctx = Arc::new(Context {
        registry: RwLock::new(PhotoStorageRegistry::new(&config.ingest.dir)),
        service: ServiceContext::try_from_config(&config)?,
        federation: FederationContext::new(&config.ingest.dir, config.federation),
        ingest: IngestContext::new(config.ingest),
        processor: ProcessorContext::new(config.image),
        base: config.base,
        event_tx,
    });

    tokio::try_join!(
        async { tokio::spawn(infra::api::run(ctx.clone())).await.unwrap() },
        async { tokio::spawn(processor::run(ctx.clone(), event_rx)).await.unwrap() },
    ).unwrap();

    Ok(())
}
