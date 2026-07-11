use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};

use crate::{api::error::ApiError, auth::{config::Entity, protocol::{extract_from_cookie, extract_from_header}, session::{Session, ValidSession}}};

pub struct IrisSession(pub Session);
pub struct ValidIrisSession(pub ValidSession);
pub struct ValidUserSession(pub ValidSession);

impl<S> FromRequestParts<S> for IrisSession
where
    S: Send + Sync,
{
    type Rejection = ApiError;

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
        };

        let session_id = match session_id {
            Some(id) => id,
            None => {
                return if ctx.auth.config.unrestricted_instance == Some(true) {
                    Ok(IrisSession(Session::Bypassed))
                } else {
                    Err(ApiError::Unauthorized("The provided session is not valid or expired".to_string()))
                };
            }
        };

        let Some(session) = super::auth::verify_session(&ctx.auth, session_id).await.unwrap() else {
            return Err(ApiError::Unauthorized("The provided session is not valid or expired".to_string()))
       };

        Ok(IrisSession(Session::Valid(session)))
    }
}

impl<S> FromRequestParts<S> for ValidIrisSession
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = IrisSession::from_request_parts(parts, state).await?;

        match session.0 {
            Session::Valid(valid_session) => Ok(Self(valid_session)),
            Session::Bypassed => Err(ApiError::Unauthorized("This endpoint requires a valid session".to_string()))
        }
    }
}

impl<S> FromRequestParts<S> for ValidUserSession
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ValidIrisSession(session) = ValidIrisSession::from_request_parts(parts, state).await?;

        match session.entity {
            Entity::User(_) => Ok(Self(session)),
            Entity::Federation(_) => Err(ApiError::Forbidden("This endpoint is not available to federation entities".to_string()))
        }
    }
}
