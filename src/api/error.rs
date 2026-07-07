use std::fmt::Display;

use axum::{Json, response::IntoResponse};
use http::StatusCode;

use crate::infra::api::types::client_error;

pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    PayloadTooLarge(String),
    TooManyRequests(String),
    InternalError(String),
}
pub type ApiErrorKind = fn(String) -> ApiError;

impl ApiError {
    pub fn passthrough<T: ToString>(kind: ApiErrorKind) -> impl FnOnce(T) -> Self {
        move |err| kind(err.to_string())
    }

    pub fn with_ctx<T: Display>(
        kind: ApiErrorKind,
        context: impl Display,
    ) -> impl FnOnce(T) -> Self {
        move |err| kind(format!("{context}\n{err}"))
    }

    pub fn internal<T: ToString>(err: T) -> Self {
        ApiError::InternalError(err.to_string())
    }

    pub fn internal_during<T: Display>(context: impl Display) -> impl FnMut(T) -> Self {
        move |err| {
            ApiError::InternalError(format!(
                "there was an internal error during {context}\n{err}"
            ))
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, reason) = match self {
            Self::BadRequest(reason) => (StatusCode::BAD_REQUEST, reason),
            Self::NotFound(reason) => (StatusCode::NOT_FOUND, reason),
            Self::Unauthorized(reason) => (StatusCode::UNAUTHORIZED, reason),
            Self::Forbidden(reason) => (StatusCode::FORBIDDEN, reason),
            Self::Conflict(reason) => (StatusCode::CONFLICT, reason),
            Self::PayloadTooLarge(reason) => (StatusCode::PAYLOAD_TOO_LARGE, reason),
            Self::TooManyRequests(reason) => (StatusCode::TOO_MANY_REQUESTS, reason),
            Self::InternalError(reason) => (StatusCode::INTERNAL_SERVER_ERROR, reason),
        };
        (status, Json(client_error(reason))).into_response()
    }
}
