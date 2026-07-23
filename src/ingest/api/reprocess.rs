use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context, api::error::ApiError, auth::{extractor::ValidUserSession, whitelist}, event::Event, infra::api::types::{SuccessfulResponse, success}, model::Identifier
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ReprocessResponse;

/// Reprocess the photo
///
/// Reprocess the photos. This fills up the missing images.
/// You need to be logged in to use this endpoint.
#[utoipa::path(
    put,
    path = "/{photo_id}/images",
    params(("photo_id" = String, Path)),
    security(
        ("session_header" = []),
        ("session_cookie" = [])
    ),
    responses(
        (status = CREATED, description = "The photo had unprocessed image(s) and was created", body = SuccessfulResponse<ReprocessResponse>),
        (status = NO_CONTENT, description = "All defined images were already creted"),
    )
)]
pub async fn reprocess(
    State(ctx): State<Arc<Context>>,
    ValidUserSession(session): ValidUserSession,
    Path((photo_id,)): Path<(Identifier,)>,
) -> Result<impl IntoResponse, ApiError> {
    whitelist::ensure_photo_allowed(&ctx.auth, &session.clone().into(), &photo_id)
        .await
        .map_err(ApiError::passthrough(ApiError::Forbidden))?;

    tracing::debug!("Loading the image");

    let photo = {
        let registry = ctx.registry.read().await;
        registry
            .load_photo(&photo_id)
            .await
            .map_err(ApiError::internal_during("reading the photo metafile"))?
    };

    if photo.is_none() {
        return Err(ApiError::NotFound(
            "the photo with the ID is not found".to_string(),
        ));
    }

    ctx.event_tx
        .send(Event::PhotoReprocessRequested { photo_id: photo_id.clone() })
        .await
        .map_err(ApiError::internal_during("queueing the reprocess event"))?;

    Ok((StatusCode::CREATED, Json(success(ReprocessResponse))))
}
