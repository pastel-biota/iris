use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = parse_config().unwrap();

    let ctx = Arc::new(Context {
        app_context: AppContext {
            dir: PathBuf::from("./_ignored/"),
        },
        registry: RwLock::new(PhotoStorageRegistry::new(Path::new("./_ignored/"))),
        service: build_service_context(&config).unwrap(),
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
        .layer(CorsLayer::permissive().allow_origin(["http://localhost:5173".parse().unwrap()]));

    axum::serve(TcpListener::bind("localhost:8080").await.unwrap(), router)
        .await
        .unwrap();
}
