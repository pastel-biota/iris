pub mod middleware;
pub mod types;
pub mod openapi;

use std::sync::Arc;

use anyhow::Context as _;
use axum::{Extension, http::StatusCode, routing::get};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use utoipa_axum::router::OpenApiRouter;

use utoipa_redoc::{Redoc, Servable as _};

use crate::infra::api::openapi::finalize_openapi;

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

    let router = OpenApiRouter::<Arc<crate::Context>>::new()
        .nest("/auth", crate::auth::api::auth_route(ctx.clone()))
        .nest("/photos", crate::ingest::api::photo_route(ctx.clone()));

    #[cfg(feature = "federation")]
    let router = router.nest("/federation", crate::federation::api::federation_route(ctx.clone()));

    let (router, openapi) = router
        .with_state(ctx.clone())
        .split_for_parts();
    let openapi = finalize_openapi(openapi);

    let router = router
        .route(
            "/openapi.json",
            get({
                let openapi = openapi.clone();
                async move || (StatusCode::OK, openapi.to_pretty_json().unwrap())
            }),
        )
        .merge(Redoc::with_url("/docs", openapi))
        .layer(CorsLayer::permissive().allow_origin(cors_origin))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(Extension(ctx.clone()));

    tracing::info!("Iris will be serving at http://{}", &ctx.ingest.config.listen);

    axum::serve(TcpListener::bind(&ctx.ingest.config.listen).await?, router).await?;

    Ok(())
}
