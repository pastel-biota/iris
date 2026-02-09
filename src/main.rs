use std::{path::{Path, PathBuf}, sync::Arc};

use tokio::{net::TcpListener, sync::{Mutex, RwLock}};
use utoipa_axum::router::OpenApiRouter;
use utoipa_redoc::{Redoc, Servable};

use crate::{context::AppContext, infra::registry::PhotoStorageRegistry, route::photo_route};

mod route;
mod infra;
pub mod model;
mod context;

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
        registry: RwLock::new(
            PhotoStorageRegistry::new(&Path::new("./_ignored/")),
        )
    });

    let (router, openapi) = OpenApiRouter::new()
        .nest("/photos", photo_route(ctx.clone()))
        .split_for_parts();

    let router = router.merge(Redoc::with_url("/docs", openapi));

    axum::serve(TcpListener::bind("127.0.0.1:8080").await.unwrap(), router).await.unwrap();
}
