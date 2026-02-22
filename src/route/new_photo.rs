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
    services::{process::process_image, property::process_properties, resize::resize_images},
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

    tracing::info!("Beginning registeration");

    let processed_image = process_image(&bytes).await.unwrap();

    {
        let mut registry = ctx.registry.write().await;

        if registry
            .image_exists_with_hash(&processed_image.sha256)
            .unwrap()
        {
            return (StatusCode::CONFLICT, Json(client_error("hash conflicted"))).into_response();
        }
    }

    let properties =
        process_properties(&ctx.service.proceessor, processed_image.image_property).unwrap();

    let photo_id = Identifier::new(&processed_image.shot_time, &ulid::Ulid::new().to_string());

    tracing::info!("Starting resize");
    let resized = resize_images(&bytes).await.unwrap();
    let resized = resized
        .resized
        .into_iter()
        .map(|(target, resized)| {
            (ImageMeta {
                width: target.w,
                height: target.h,
                extension: target.ext.extensions_str()[0].to_string(),
                image_id: target.id.to_string(),
            }, resized)
        })
        .collect::<Vec<_>>();

    let photo = PhotoMeta {
        id: photo_id.clone(),
        images: resized.iter().map(|(img, _)| img.clone()).collect(),
        original_sha256: processed_image.sha256,
        shot_time: processed_image.shot_time,
        properties,
    };

    let mut registry = ctx.registry.write().await;
    registry.new_photo(&photo).unwrap();

    for resized in resized {
        registry
            .upload_image(&photo.id, &resized.0.image_id, &resized.0.extension, &resized.1)
            .await
            .unwrap();
    }

    let response = NewPhotoResponse {
        photo: photo.into(),
    };

    (StatusCode::CREATED, Json(success(response))).into_response()
}
