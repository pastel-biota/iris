use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{http::StatusCode, routing::get};
use tokio::{
    net::TcpListener,
    sync::RwLock,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_redoc::{Redoc, Servable};

use crate::{
    context::AppContext,
    infra::registry::PhotoStorageRegistry,
    route::photo_route,
};

mod context;
mod infra;
pub mod model;
mod route;

pub struct Context {
    pub app_context: AppContext,
    pub registry: RwLock<PhotoStorageRegistry>,
}

#[tokio::main]
async fn main() {
    let ctx = Arc::new(Context {
        app_context: AppContext {
            dir: PathBuf::from("./_ignored/"),
        },
        registry: RwLock::new(PhotoStorageRegistry::new(Path::new("./_ignored/"))),
    });

    let (router, openapi) = OpenApiRouter::new()
        .nest("/photos", photo_route(ctx.clone()))
        .split_for_parts();

    let router = router.route(
        "/openapi.json",
        get({
            let openapi = openapi.clone();
            async move || (StatusCode::OK, openapi.to_pretty_json().unwrap())
        }),
    );

    let router = router.merge(Redoc::with_url("/docs", openapi));

    axum::serve(TcpListener::bind("127.0.0.1:8080").await.unwrap(), router)
        .await
        .unwrap();
}
