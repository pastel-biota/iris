use std::sync::Arc;

use axum::{Json, extract::{Path, State}, http::StatusCode, response::IntoResponse};

use crate::{Context, auth::extractor::IrisSession, infra::api::types::{ClientError, SuccessfulResponse, client_error, success}, model::Identifier};

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
    Path((photo_id,)): Path<(String,)>,
    IrisSession(_): IrisSession,
) -> impl IntoResponse {
    let Ok(photo_id) = photo_id.parse::<Identifier>() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(client_error("Photo id is not valid as the Id")),
        )
            .into_response();
    };

    let mut registry = ctx.registry.write().await;
    registry.unregister(&photo_id).unwrap();

    (StatusCode::NO_CONTENT).into_response()
}
