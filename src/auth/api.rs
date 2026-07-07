use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use axum_limit::Quota;
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{api::error::ApiError, auth::{auth::LoginError, endpoint, extractor::ValidIrisSession, password::Password}, infra::api::{rate_limit::{self, ClientIp}, types::{ClientError, SuccessfulResponse, client_error, success}}};

pub fn auth_route(_ctx: Arc<crate::Context>) -> OpenApiRouter<Arc<crate::Context>> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(me))
}

/// Log in to Iris' user
///
/// Issue a new session for Iris to access to the administrative endpoints.
/// You need to create users in advance to log in.
#[utoipa::path(
    post,
    path = "/login",
    request_body(content = endpoint::LoginBody, content_type = "application/json"),
    responses(
        (status = OK, description = "The user has been successfully logged in and the session is issued", body = SuccessfulResponse<endpoint::LoginResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
        (status = UNAUTHORIZED, description = "The provided credential is not valid", body = ClientError),
    )
)]
async fn login(
    State(ctx): State<Arc<crate::Context>>,
    ip: ClientIp,
    Json(login): Json<endpoint::LoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Tighter than the global limit: login is a brute-force target.
    rate_limit::enforce(&ctx, ip, Quota::new(5, 60_000)).await?;

    let login = super::auth::login_to_entity(&ctx.auth, &login.username, &Password::from_string(login.password)).await;

    let session_key = match login {
        Ok(key) => key,
        Err(LoginError::InvalidCredential) => {
            return Ok((StatusCode::UNAUTHORIZED, Json(client_error("Incorrect username / password"))).into_response());
        },
        Err(LoginError::GenericError(e)) => {
            return Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(client_error(&e.to_string()))).into_response());
        }
    };

    Ok((
        StatusCode::OK,
        [(http::header::SET_COOKIE, super::protocol::create_cookie(&session_key, &ctx.base.host))],
        Json(success(endpoint::LoginResponse { session_key })),
    )
        .into_response())
}

/// Retrieve the information about currently logged in user.
#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = OK, description = "The user has been successfully logged in and the session is issued", body = SuccessfulResponse<endpoint::MeResponse>),
        (status = UNAUTHORIZED, description = "The user is not logged in", body = ClientError),
    )
)]
async fn me(ValidIrisSession(session): ValidIrisSession) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(success(endpoint::MeResponse { name: session.entity.name().clone() })),
    )
        .into_response()
}
