use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

pub struct IrisSignature(pub Option<String>);

impl<S> FromRequestParts<S> for IrisSignature
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let signature = parts
            .headers
            .get("x-iris-signature")
            .and_then(|v| v.to_str().ok());

        Ok(IrisSignature(signature.map(|sig| sig.to_string())))
    }
}
