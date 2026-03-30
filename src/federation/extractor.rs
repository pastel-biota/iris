use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

pub struct RequestingInstance(pub String);

impl<S> FromRequestParts<S> for RequestingInstance
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get("x-iris-requesting-instance")
            .and_then(|v| v.to_str().ok())
            .map(|v| RequestingInstance(v.to_string()))
            .ok_or(StatusCode::BAD_REQUEST)
    }
}
