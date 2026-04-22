use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

use crate::auth::{protocol::{extract_from_cookie, extract_from_header}, session::Session};

pub struct IrisSession(pub Session);

impl<S> FromRequestParts<S> for IrisSession
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(ctx) = parts.extensions.get::<Arc<crate::Context>>() else {
            panic!("The context was not provided through the extension");
        };

        let authorization = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok());

        let cookies = parts
            .headers
            .get("cookie")
            .and_then(|v| v.to_str().ok());

        let session_id = {
            if let Some(authorization) = authorization {
                extract_from_header(authorization)?
            } else if let Some(cookies) = cookies {
                extract_from_cookie(cookies)
            } else {
                None
            }
        }.ok_or(StatusCode::UNAUTHORIZED)?;

        let Some(session) = super::auth::verify_session(&ctx.auth, session_id).await.unwrap() else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        Ok(IrisSession(session))
    }
}
