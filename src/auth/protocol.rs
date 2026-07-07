use axum::response::{Response, IntoResponse};

use crate::{api::error::ApiError, auth::session::SESSION_DURATION, infra::api::types::client_error};

pub const SESSION_COOKIE: &str = "session_id";

pub fn extract_from_header(authorization: &str) -> Result<Option<&str>, ApiError> {
    let Some((schema, value)) = authorization.split_once(" ") else {
        return Err(ApiError::BadRequest("Expected bearer token".to_string()));
    };

    if schema.to_lowercase() != "bearer" {
        return Err(ApiError::BadRequest("The authorization bearer is not bearer".to_string()));
    };

    Ok(Some(value.trim()))
}

pub fn create_cookie(session_key: &str, domain: &str) -> String {
    let max_age = SESSION_DURATION.num_seconds();
    format!("{SESSION_COOKIE}={session_key}; Domain={domain}; Max-Age={max_age}; HttpOnly; SameSite=Lax")
}

pub fn extract_from_cookie(cookie: &str) -> Option<&str> {
    cookie
        .split(";")
        .flat_map(|content| content.trim().split_once("="))
        .find(|(key, _)| key == &SESSION_COOKIE)
        .map(|(_, value)| value)
}

