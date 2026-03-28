use std::sync::Arc;

use anyhow::Context as _;
use axum::{http::StatusCode, routing::get};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use utoipa_axum::router::OpenApiRouter;

use route::photo_route;
use utoipa_redoc::{Redoc, Servable as _};

use crate::ingest::config::IngestConfig;

pub mod config;
mod middleware;
mod route;
pub mod technicals;

pub struct IngestContext {
    pub config: IngestConfig,
}

impl IngestContext {
    pub fn new(config: IngestConfig) -> Self {
        Self { config }
    }
}

pub async fn run(
    ctx: Arc<crate::Context>,
) -> Result<(), anyhow::Error> {
    let cors_origin = ctx.ingest.config
        .cors_origin
        .iter()
        .map(|origin| {
            origin
                .parse()
                .with_context(|| format!("The CORS origin is not valid: {}", origin))
        })
        .collect::<Result<Vec<_>, _>>()?;

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
            tower::ServiceBuilder::new()
                .layer(CorsLayer::permissive().allow_origin(cors_origin))
                .layer(axum::middleware::from_fn(middleware::access_log))
        );

    tracing::info!("Iris will be serving at http://{}", &ctx.ingest.config.listen);

    axum::serve(TcpListener::bind(&ctx.ingest.config.listen).await?, router).await?;

    Ok(())
}

