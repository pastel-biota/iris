mod ping;
mod list;

use std::{str::FromStr, sync::Arc};

use axum::{body::{Body, to_bytes}, extract::{OriginalUri, Request, State}, middleware::{self, Next}, response::Response};
use ed25519_dalek::{Signature, Verifier, VerifyingKey, pkcs8::DecodePublicKey as _};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{Context, util::collect_n};

pub fn federation_route(ctx: Arc<Context>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(ping::ping))
        .routes(routes!(list::list))
        .with_state(ctx.clone())
        .route_layer(middleware::from_fn_with_state(ctx, verify_hash))
}

#[axum::debug_middleware]
async fn verify_hash(
    State(ctx): State<Arc<Context>>,
    uri: OriginalUri,
    req: Request,
    next: Next
) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();

    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();

    let body_bytes: Option<&[u8]> = if bytes.is_empty() {
        None
    } else {
        Some(&bytes[..])
    };

    let hash = super::auth::create_hash(
        &parts.method,
        uri.path(),
        uri.query(),
        body_bytes
    ).unwrap();

    let Some(header) = parts.headers.get("x-iris-signature") else {
        return Ok(next.run(Request::from_parts(parts, Body::from(bytes))).await);
    };

    let Some([host, provided_hash, challenge]) = collect_n(header.to_str().unwrap().splitn(3, ":")) else {
        panic!("The hash value was not in the expected format: {}", header.to_str().unwrap());
    };

    let verify_key = ctx.federation.config.hosts[host].pubkey.trim();
    let verify_key = VerifyingKey::from_public_key_pem(verify_key).unwrap();

    verify_key.verify(
        provided_hash.as_bytes(),
        &Signature::from_str(challenge).unwrap()
    ).unwrap();

    dbg!(header.to_str().unwrap());
    assert!(&hash == provided_hash);

    Ok(next.run(Request::from_parts(parts, Body::from(bytes))).await)
}
