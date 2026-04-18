use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};
use http::StatusCode;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{auth::{auth::LoginError, password::Password}, infra::api::types::{ClientError, SuccessfulResponse, client_error, success}};

pub fn auth_route(ctx: Arc<crate::Context>) -> OpenApiRouter<Arc<crate::Context>> {
    OpenApiRouter::new()
        .routes(routes!(login))
}

#[derive(Clone, Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct LoginBody {
    username: String,
    password: Password,
}


#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    session_key: String,
}
/// Log in to Iris' user
///
/// Issue a new session for Iris to access to the administrative endpoints.
/// You need to create users in advance to log in.
#[utoipa::path(
    post,
    path = "/login",
    request_body(content = LoginBody, content_type = "application/octet-stream"),
    responses(
        (status = OK, description = "The user has been successfully logged in and the session is issued", body = SuccessfulResponse<LoginResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
        (status = UNAUTHORIZED, description = "The provided credential is not valid", body = ClientError),
    )
)]
async fn login(
    State(ctx): State<Arc<crate::Context>>,
    Json(login): Json<LoginBody>,
) -> impl IntoResponse {
    let login = super::auth::login_to_user(&ctx.auth, &login.username, &login.password).await;

    let session_key = match login {
        Ok(key) => key,
        Err(LoginError::InvalidCredential) => {
            return (StatusCode::UNAUTHORIZED, Json(client_error("Incorrect username / password"))).into_response();
        },
        Err(LoginError::GenericError(e)) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(client_error(&e.to_string()))).into_response();
        }
    };

    (
        StatusCode::OK,
        [(http::header::SET_COOKIE, super::protocol::create_cookie(&session_key, &ctx.base.host))],
        Json(success(LoginResponse { session_key })),
    )
        .into_response()
}
