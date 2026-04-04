mod ping;
mod list;

use std::sync::Arc;

use axum::{body::{Body, to_bytes}, extract::{OriginalUri, Request, State}, middleware::{self, Next}, response::Response};
use http::StatusCode;
use image::EncodableLayout;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{Context, federation::{auth::{self, ChallengePayload}, extractor::IrisSignature}};

pub fn federation_route(ctx: Arc<Context>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(ping::ping))
        .routes(routes!(list::list))
        .with_state(ctx.clone())
        .route_layer(middleware::from_fn_with_state(ctx, verify_hash))
}

#[derive(Clone)]
pub struct IrisHost(pub String);

async fn verify_hash(
    State(ctx): State<Arc<Context>>,
    IrisSignature(sig): IrisSignature,
    uri: OriginalUri,
    req: Request,
    next: Next
) -> Result<Response, StatusCode> {
    let Some(sig) = sig else {
        if req.uri().path() == "/ping" {
            return Ok(next.run(req).await);
        };
        return Err(StatusCode::FORBIDDEN);
    };

    let (mut parts, body) = req.into_parts();
    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();

    let host = auth::verify::get_sender_host(&sig).unwrap();

    auth::verify::verify_challenge(
        &sig,
        ctx.federation.config.hosts[host].pubkey.trim(),
        &ChallengePayload {
            host,
            method: &parts.method,
            path_name: uri.path(),
            query: uri.query(),
            body: Some(bytes.as_bytes()),
        },
    ).unwrap();

    parts.extensions.insert(IrisHost(host.to_string()));

    Ok(next.run(Request::from_parts(parts, Body::from(bytes))).await)
}
