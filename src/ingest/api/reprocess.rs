use std::{io::Cursor, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use image::ImageReader;

use crate::{
    Context, auth::extractor::IrisSession, infra::api::types::{SuccessfulResponse, client_error, success}, model::Identifier
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

    tracing::debug!("Starting the resize");

    // let targets = RESIZE_TARGETS
    //     .iter()
    //     .filter(|target| !photo.images.keys().any(|image_id| target.id == image_id))
    //     .collect::<Vec<&_>>();

    // if targets.is_empty() {
    //     return StatusCode::NO_CONTENT.into_response();
    // }

    tracing::debug!("Reading the image");

    let original_photo = {
        let mut registry = ctx.registry.write().await;
        registry.load_original_image(&photo_id).await.unwrap()
    };

    tracing::debug!("Image was read. Decoding");

    let _original_photo = ImageReader::new(Cursor::new(original_photo))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    tracing::debug!("Decoded! Resizing");

    // let resized = resize_images(original_photo, targets).await.unwrap();

    // let mut registry = ctx.registry.write().await;
    // for resized in resized.resized {
    //     registry
    //         .upload_image(&photo_id, &resized.target.id, &resized.meta, &resized.data)
    //         .await
    //         .unwrap();
    // }

    (StatusCode::CREATED, Json(success(ReprocessResponse))).into_response()
}
