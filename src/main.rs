use std::{process::ExitCode, sync::Arc};

use anyhow::Context as _;
use axum::{http::StatusCode, routing::get};
use tokio::{net::TcpListener, sync::RwLock};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;
use utoipa_axum::router::OpenApiRouter;
use utoipa_redoc::{Redoc, Servable};

use crate::{
    config::parse_config,
    context::AppContext,
    infra::registry::PhotoStorageRegistry,
    route::photo_route,
    services::{ServiceContext, build_service_context},
};

pub mod config;
mod context;
mod infra;
pub mod model;
mod route;
pub mod services;

pub struct Context {
    pub app_context: AppContext,
    pub registry: RwLock<PhotoStorageRegistry>,
    pub service: ServiceContext,
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
        },
        Err(e) => {
            eprintln!("The Iris experienced fatal issue and cannot continue:");
            eprintln!("{e}");
            tracing::error!("The Iris is exiting: {e}");

            for cause in e.chain().skip(1) {
                eprintln!("  <- {cause}");
                tracing::error!("<- {cause}");
            }

            1.into()
        },
    }
}

async fn run() -> Result<(), anyhow::Error> {
    let config = parse_config()?;

    let ctx = Arc::new(Context {
        app_context: AppContext {
            dir: config.dir.clone(),
        },
        registry: RwLock::new(PhotoStorageRegistry::new(&config.dir)),
        service: build_service_context(&config)?,
    });

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
        .layer(CorsLayer::permissive().allow_origin(
            config
                .cors_origin
                .iter()
                .map(|origin| origin.parse().with_context(|| format!("The CORS origin is not valid: {}", origin)))
                .collect::<Result<Vec<_>, _>>()?
        ));

    tracing::info!("Iris will be serving at http://{}", &config.listen);

    axum::serve(TcpListener::bind(&config.listen).await?, router)
        .await?;

    Ok(())
}
