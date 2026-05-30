use std::{io::Cursor, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use image::ImageReader;

use crate::{
    Context, auth::extractor::IrisSession, event::Event, infra::api::types::{SuccessfulResponse, client_error, success}, model::Identifier
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
) -> impl IntoResponse {
    let Ok(photo_id) = photo_id.parse::<Identifier>() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(client_error("Photo id is not valid as the Id")),
        )
            .into_response();
    };

    tracing::debug!("Loading the image");

    let photo = {
        let mut registry = ctx.registry.write().await;
        match registry.load_photo(&photo_id).await {
            Ok(photo) => photo,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(client_error(&format!(
                        "there was an internal error during reading the photo metafile: {:#?}",
                        err
                    ))),
                )
                    .into_response();
            }
        }
    };

    let Some(_photo) = photo else {
        return (
            StatusCode::NOT_FOUND,
            Json(client_error("the photo with the ID is not found")),
        )
            .into_response();
    };

    ctx.event_tx
        .send(Event::PhotoReprocessRequested { photo_id: photo_id.clone() })
        .await
        .unwrap();

    (StatusCode::CREATED, Json(success(ReprocessResponse))).into_response()
}
