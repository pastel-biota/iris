use std::sync::Arc;

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};

use crate::{Context, api::error::ApiError, auth::{extractor::ValidUserSession, whitelist}, infra::api::types::ClientError, model::Identifier};

/// A new field
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    delete,
    // TODO: Replace with the correct path - use {xxx} to accept path parameter
    path = "/{photo_id}",
    params(
        ("photo_id" = String, Path),
    ),
    security(
        ("session_header" = []),
        ("session_cookie" = [])
    ),
    responses(
        (status = NO_CONTENT, description = "The photo has been removed from the Iris"),
        (status = NOT_FOUND, description = "The photo with the same ID is not found", body = ClientError),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn delete(
    State(ctx): State<Arc<Context>>,
    Path((photo_id,)): Path<(Identifier,)>,
    ValidUserSession(session): ValidUserSession,
) -> Result<impl IntoResponse, ApiError> {
    whitelist::ensure_photo_allowed(&ctx.auth, &session.clone().into(), &photo_id)
        .map_err(ApiError::passthrough(ApiError::Forbidden))?;

    let mut registry = ctx.registry.write().await;
    registry
        .unregister(&photo_id)
        .map_err(ApiError::internal_during("unregistering the photo"))?;

    Ok(StatusCode::NO_CONTENT)
}
