use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::TryStreamExt as _;
use tokio_util::io::StreamReader;

use crate::{
    Context,
    model::Identifier,
    route::{BinaryBody, ClientError, SuccessfulResponse, client_error, photo_route, success},
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UploadImageResponse {
    /// Identifier assigned to the created photo.
    #[schema(example = "202601_img_0001_jpg-01AAAA")]
    id: String,

    /// How much of parallel upload is accepted for the upload of this image.
    max_parallelism: u8,
}

/// Registers a new photo
///
/// Register a new photo, and prepare for the upload for the actual image.
#[utoipa::path(
    post,
    path = "/{photo_id}/images/{image_id}",
    params(
        ("photo_id" = String, Path),
        ("image_id" = String, Path),
    ),
    request_body(content = BinaryBody, content_type = "application/octet-stream"),
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
            return (
                StatusCode::BAD_REQUEST,
                Json(client_error("The photo_id has malformed format")),
            )
                .into_response();
        }
    };

    let mut registry = ctx.registry.write().await;
    let mut reader = StreamReader::new(body.into_data_stream().map_err(std::io::Error::other));

    registry
        .upload_image(&photo_id, &image_id, "jpg", &mut reader)
        .await
        .unwrap();

    (StatusCode::OK, Json(success(
        UploadImageResponse {
            id: photo_id.to_string(),
            max_parallelism: 4,
        }
    ))).into_response()
}
