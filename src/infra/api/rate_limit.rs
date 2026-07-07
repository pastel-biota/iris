use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, FromRequestParts, Request},
    http::{HeaderMap, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_limit::{Key, LimitState, Quota, StorageKey};

use crate::api::error::ApiError;

/// Shared rate limit backend, keyed by [`ClientIp`].
///
/// One backend can be reused by every call site: [`enforce`] takes the quota per call,
/// and axum_limit buckets its internal state per (key, quota) pair, so different quotas
/// on the same IP never collide.
pub type RateLimit = LimitState<ClientIp>;

/// The client's IP address, as seen through the Cloudflare Tunnel in front of Iris.
///
/// Trusts the `CF-Connecting-IP` header, since Cloudflare sets/overwrites it on every
/// request that reaches us through the tunnel. Falls back to the raw TCP peer address
/// when the header is absent (local dev without Cloudflare in front) -- that fallback is
/// spoofable and must not be trusted if Iris is ever exposed directly to the internet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ClientIp(pub IpAddr);

impl Key for ClientIp {
    type Extractor = ClientIp;

    fn from_extractor(extractor: &Self::Extractor) -> Self {
        *extractor
    }
}

impl StorageKey for ClientIp {
    fn storage_key(&self) -> String {
        self.0.to_string()
    }
}

impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        client_ip(&parts.headers, parts.extensions.get()).map(ClientIp).ok_or_else(|| {
            ApiError::InternalError("Could not determine the client's IP address".to_string())
        })
    }
}

fn client_ip(headers: &HeaderMap, connect_info: Option<&ConnectInfo<SocketAddr>>) -> Option<IpAddr> {
    headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .or_else(|| connect_info.map(|ConnectInfo(addr)| addr.ip()))
}

/// Checks `quota` for `ip` against the shared rate limit backend on `ctx`.
///
/// A no-op when `base.rate_limit` is set to `false` in the instance config -- meant for a
/// private, trusted-network instance where the limits meant for a public instance would
/// only get in the way (e.g. bulk uploads from a script).
pub async fn enforce(ctx: &crate::Context, ip: ClientIp, quota: Quota) -> Result<(), ApiError> {
    if ctx.base.rate_limit == Some(false) {
        return Ok(());
    }

    let snapshot = ctx.rate_limit.check(ip, quota).await.map_err(ApiError::internal)?;

    if snapshot.allowed {
        Ok(())
    } else {
        Err(ApiError::TooManyRequests("Too many requests, please try again later".to_string()))
    }
}

/// Light rate limit applied to every request, keyed by client IP.
///
/// This is meant as a coarse, global backstop -- endpoints that need a stricter limit
/// (e.g. `/auth/login`) should additionally call [`enforce`] with their own quota.
pub async fn global_rate_limit(req: Request, next: Next) -> Response {
    let ctx = req
        .extensions()
        .get::<Arc<crate::Context>>()
        .expect("The context was not provided through the extension")
        .clone();

    let Some(ip) = client_ip(req.headers(), req.extensions().get()) else {
        return ApiError::InternalError("Could not determine the client's IP address".to_string())
            .into_response();
    };

    if let Err(err) = enforce(&ctx, ClientIp(ip), Quota::per_second(20)).await {
        return err.into_response();
    }

    next.run(req).await
}
