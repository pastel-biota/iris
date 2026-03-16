use std::{process::ExitCode, sync::Arc};

use anyhow::Context as _;
use axum::{http::StatusCode, routing::get};
use chrono::DateTime;
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use utoipa_axum::router::OpenApiRouter;
use utoipa_redoc::{Redoc, Servable};

use crate::{
    config::parse_config,
    context::AppContext,
    ingest::{
        infra::registry::PhotoStorageRegistry,
        model::Identifier,
        route::photo_route,
        services::{ServiceContext, build_service_context},
    },
    processor::{JobApplication, ProcessorContext, ProcessorRunner},
};

pub mod config;
mod context;
pub mod ingest;
pub mod processor;

pub struct Context {
    pub app_context: AppContext,
    pub registry: RwLock<PhotoStorageRegistry>,
    pub service: ServiceContext,
    pub processor: ProcessorContext,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
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
        app_context: AppContext {
            dir: config.dir.clone(),
        },
        processor: ProcessorContext::default(),
        registry: RwLock::new(PhotoStorageRegistry::new(&config.dir)),
        service: build_service_context(&config)?,
    });

    let processor = ProcessorRunner::from_context(&ctx.processor);

    let (router, openapi) = OpenApiRouter::new()
        .nest("/photos", photo_route(ctx.clone()))
        .split_for_parts();

    let router = router
        .route(
            "/openapi.json",
            get({
                let openapi = openapi.clone();
                async move || (StatusCode::OK, openapi.to_pretty_json().unwrap())
            }),
        )
        .merge(Redoc::with_url("/docs", openapi))
        .layer(
            CorsLayer::permissive().allow_origin(
                config
                    .cors_origin
                    .iter()
                    .map(|origin| {
                        origin
                            .parse()
                            .with_context(|| format!("The CORS origin is not valid: {}", origin))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        );

    tracing::info!("Iris will be serving at http://{}", &config.listen);

    tokio::spawn(Arc::new(processor).start());

    tokio::spawn({
        let ctx = ctx.clone();
        async move {
            for wave in 0..10 {
                for id in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                    let job = JobApplication {
                        id: id + (wave * 10),
                    };

                    println!("Notifying: {job:?}");
                    ctx.processor.add_job(job);
                }
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            }
        }
    });

    axum::serve(TcpListener::bind(&config.listen).await?, router).await?;

    Ok(())
}
