use std::{process::ExitCode, sync::Arc};

use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

use crate::{
    config::parse_config,
    ingest::{
        IngestContext, infra::registry::PhotoStorageRegistry, services::ServiceContext
    },
    processor::{ProcessorContext, register_resize},
};

pub mod config;
pub mod ingest;
pub mod processor;

pub struct Context {
    pub ingest: IngestContext,
    pub registry: RwLock<PhotoStorageRegistry>,
    pub service: ServiceContext,
    pub processor: ProcessorContext,
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

    let ctx = Arc::new(Context {
        registry: RwLock::new(PhotoStorageRegistry::new(&config.ingest.dir)),
        service: ServiceContext::try_from_config(&config)?,
        ingest: IngestContext::new(config.ingest),
        processor: ProcessorContext::new(config.image),
    });

    tokio::try_join!(
        async { tokio::spawn(ingest::run(ctx.clone())).await.unwrap() },
        async { tokio::spawn(processor::run(ctx.clone())).await.unwrap() },
        async {
            let ctx = ctx.clone();
            tokio::spawn({
                async move {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    process_image(ctx.clone());
                    Ok(())
                }
            }).await.unwrap()
        }
    ).unwrap();

    Ok(())
}

fn process_image(ctx: Arc<Context>) {
    register_resize(
        &ctx.processor,
        "202601-01KKN2KYDKFM1Y7QXTAK6B6F66".parse().unwrap(),
        "main",
    );
}

