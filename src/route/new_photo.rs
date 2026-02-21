use std::sync::Arc;

use axum::{
    Json,
    body::{Body, to_bytes},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context,
    model::{Identifier, ImageMeta, PhotoMeta},
    route::{
        BinaryBody, ClientError, SuccessfulResponse, client_error, scheme::PhotoScheme, success,
    },
    services::{process::process_image, property::process_properties},
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NewPhotoResponse {
    photo: PhotoScheme,
}

pub const MAX_BODY: usize = 100 * 1024 * 1024;

/// Registers a new photo
///
/// Register a new photo, and prepare for the upload for the actual image.
#[utoipa::path(
    post,
    path = "/",
    request_body(content = BinaryBody, content_type = "application/octet-stream"),
    responses(
        (status = CREATED, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<NewPhotoResponse>),
        (status = CONFLICT, description = "There already was a photo registered with the matching hash", body = ClientError),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn new_photo(State(ctx): State<Arc<Context>>, body: Body) -> impl IntoResponse {
    let bytes = to_bytes(body, MAX_BODY).await.unwrap();

    if bytes.len() == MAX_BODY {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(client_error(&format!(
                "The file size is restricted to under {MAX_BODY} bytes"
            ))),
        )
            .into_response();
    }

    for i in 0..3 {
        for b in 0..15 {
            print!("{:02x} ", bytes[i * 16 + b]);
        }
        print!(" | ");
        for b in 0..15 {
            let byte = bytes[i * 16 + b] as char;
            print!(
                "{}",
                if byte.is_ascii() && !byte.is_ascii_control() {
                    byte
                } else {
                    '.'
                }
            );
        }
        println!();
    }

    let processed_image = process_image(&bytes).await.unwrap();

    let mut registry = ctx.registry.write().await;

    if registry
        .image_exists_with_hash(&processed_image.sha256)
        .unwrap()
    {
        return (StatusCode::CONFLICT, Json(client_error("hash conflicted"))).into_response();
    }

    let properties =
        process_properties(&ctx.service.proceessor, processed_image.image_property).unwrap();

    let photo_id = Identifier::new(&processed_image.shot_time, &ulid::Ulid::new().to_string());
    let image_id = ulid::Ulid::new().to_string();

    let photo = PhotoMeta {
        id: photo_id.clone(),
        images: vec![ImageMeta {
            name: "original".to_string(),
            width: 1920,
            height: 1080,
            image_id: image_id.clone(),
            extension: "jpg".to_string(),
        }],
        original_sha256: processed_image.sha256,
        shot_time: processed_image.shot_time,
        properties,
    };

    registry.new_photo(&photo).unwrap();
    registry
        .upload_image(&photo.id, &image_id, "jpg", &bytes)
        .await
        .unwrap();

    let response = NewPhotoResponse {
        photo: photo.into(),
    };

    (StatusCode::CREATED, Json(success(response))).into_response()
}
