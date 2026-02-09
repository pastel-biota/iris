use std::sync::Arc;

use axum::{Json, body::{Body, BodyDataStream}, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset};
use futures_util::TryStreamExt as _;
use tokio_util::io::StreamReader;

use crate::{Context, infra::meta::{ImageMeta, PhotoMeta, PropertiesMeta}, model::Identifier, route::{ClientError, SuccessfulResponse, client_error, success}};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UploadImageResponse {
    /// Identifier assigned to the created photo.
    #[schema(example = "202601_img_0001_jpg-01AAAA")]
    id: String,

    /// The list of identifiers assigned to the specified images.
    /// The image ID is used to upload the actual image later.
    images: Vec<UploadImageImageResponse>,

    /// How much of parallel upload is accepted for the upload of this image.
    max_parallelism: u8,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UploadImageImageResponse {
    #[schema(example = "1080p")]
    name: String,

    #[schema(example = "01AAAA")]
    image_id: String
}

/// Registers a new photo
///
/// Register a new photo, and prepare for the upload for the actual image.
#[utoipa::path(
    post,
    path = "/images/{photo_id}/{image_id}",
    params(
        ("photo_id" = String, Path),
        ("image_id" = String, Path),
    ),
    request_body(content_type = "image/*"),
    responses(
        (status = OK, description = "The image was uploaded.", body = SuccessfulResponse<UploadImageResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn upload_image(
    State(ctx): State<Arc<Context>>,
    Path((photo_id, image_id)): Path<(String, String)>,
    body: Body,
) -> impl IntoResponse {
    let photo_id = match photo_id.parse::<Identifier>() {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(client_error("The photo_id has malformed format")))
                .into_response();
        },
    };

    let mut registry = ctx.registry.write().await;
    let mut reader = StreamReader::new(
        body.into_data_stream()
            .map_err(std::io::Error::other)
    );

    registry.upload_image(&photo_id, &image_id, "jpg", &mut reader)
        .await
        .unwrap();

    (StatusCode::CREATED, Json(success(()))).into_response()
}
