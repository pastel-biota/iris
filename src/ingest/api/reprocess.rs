use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context, api::{extract::parse_identifier, error::ApiError}, auth::extractor::IrisSession, event::Event, infra::api::types::{SuccessfulResponse, success}
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
    _: IrisSession,
    Path((photo_id,)): Path<(String,)>,
) -> Result<impl IntoResponse, ApiError> {
    let photo_id = parse_identifier(&photo_id)?;

    tracing::debug!("Loading the image");

    let photo = {
        let mut registry = ctx.registry.write().await;
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
