use http::StatusCode;

use crate::auth::session::SESSION_DURATION;

pub const SESSION_COOKIE: &str = "session_id";

pub fn extract_from_header(authorization: &str) -> Result<Option<&str>, StatusCode> {
    let Some((schema, value)) = authorization.split_once(" ") else {
        return Err(StatusCode::BAD_REQUEST);
    };

    if schema.to_lowercase() != "bearer" {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
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

