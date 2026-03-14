use std::sync::Arc;

use axum::{
    Json,
    body::{Body, to_bytes},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context, infra::registry::NewPhotoParam, model::Identifier, route::{
        BinaryBody, ClientError, SuccessfulResponse, client_error, scheme::PhotoScheme, success,
    }, services::{process::{get_hash, process_image}, property::process_properties, resize::{RESIZE_TARGETS, resize_images}}
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

    let sha256 = get_hash(&bytes);

    {
        let mut registry = ctx.registry.write().await;

        if registry.image_exists_with_hash(&sha256).unwrap() {
            return (StatusCode::CONFLICT, Json(client_error("hash conflicted"))).into_response();
        }
    }

    let processed = process_image(&bytes).await.unwrap();
    let properties = process_properties(&ctx.service.proceessor, processed.image_property).unwrap();

    let photo_id = Identifier::new(&processed.shot_time, &ulid::Ulid::new().to_string());

    let original_ext = processed.original_meta.extension.clone();

    let new_photo = NewPhotoParam {
        id: photo_id.clone(),
        original: processed.original_meta,
        original_sha256: sha256,
        shot_time: processed.shot_time,
        representative_rgb: processed.averaged_color.0,
        properties,
    };

    let new_photo = {
        let mut registry = ctx.registry.write().await;
        let new_photo = registry.new_photo(new_photo).unwrap();
        registry
            .upload_original_image(&photo_id, &original_ext, &bytes).await.unwrap();
        registry
            .upload_image(
                &photo_id,
                processed.instant_image.target.id,
                &processed.instant_image.meta,
                &processed.instant_image.data,
            ).await.unwrap();

        new_photo
    };

    tracing::info!("Starting resize");

    let resized = resize_images(processed.original_image, RESIZE_TARGETS[1..=3].iter().collect()).await.unwrap();

    let mut registry = ctx.registry.write().await;

    let mut photo = new_photo;
    for resized in resized.resized {
        photo = registry
            .upload_image(&photo_id, &resized.target.id, &resized.meta, &resized.data)
            .await
            .unwrap();
    }

    let response = NewPhotoResponse {
        photo: photo.into(),
    };

    (StatusCode::CREATED, Json(success(response))).into_response()
}
