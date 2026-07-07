use std::sync::Arc;

use axum::{
    Json,
    body::{Body, to_bytes},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::TryStreamExt;
use http::request::Parts;

use crate::{
    Context, api::error::ApiError, auth::extractor::ValidUserSession, event::Event, infra::api::types::{
        BinaryBody, ClientError, SuccessfulResponse, success,
    }, ingest::{
        api::scheme::PhotoScheme,
        technicals::{image::{
            process::{get_hash, process_image},
            property::process_properties,
        }, stream::SizedStream},
    }, model::Identifier, repository::registry::NewPhotoParam,
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NewPhotoResponse {
    photo: PhotoScheme,
}

pub const MAX_BODY: usize = 100 * 1024 * 1024;

/// Registers a new photo
///
/// Upload a photo payload and register. This triggers the image processing.
/// Note that the response from this endpoint does not guarantee (and practically does not mean)
/// that the image has been generated. However representative_rgb is available, and the client can
/// use this value to show the placeholder for the uploaded photo, until the image is being
/// processed.
///
/// You need to be logged in to use this endpoint.
#[utoipa::path(
    post,
    path = "/",
    security(
        ("session_header" = []),
        ("session_cookie" = [])
    ),
    request_body(content = BinaryBody, content_type = "application/octet-stream"),
    responses(
        (status = CREATED, description = "The photo was registered and the image processing was queued.", body = SuccessfulResponse<NewPhotoResponse>),
        (status = CONFLICT, description = "There already was a photo registered with the matching hash", body = ClientError),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn new_photo(
    State(ctx): State<Arc<Context>>,
    ValidUserSession(_): ValidUserSession,
    parts: Parts,
    body: Body
) -> Result<impl IntoResponse, ApiError> {
    let Some(length) = parts.headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return Err(ApiError::BadRequest("Content-Length needs to be provided".to_owned()));
    };

    let received = ctx.ingest.sized_sream
        .receive_stream(length, body.into_data_stream())
        .await
        .map_err(ApiError::internal_during("reading the body content"))?;
    let content = received.read().await
        .map_err(ApiError::internal_during("reading the body content"))?;
    let bytes = &content.bytes;

    tracing::info!("Beginning registeration");

    let sha256 = get_hash(&bytes);

    {
        tracing::debug!("Retrieving registry for verifying the conflict");
        let mut registry = ctx.registry.write().await;
        tracing::debug!("Retrieved registry");

        if registry
            .image_exists_with_hash(&sha256)
            .map_err(ApiError::internal_during("checking the hash conflict"))?
        {
            return Err(ApiError::Conflict("hash conflicted".to_string()));
        }
    }

    let processed = process_image(bytes.clone())
        .await
        .map_err(ApiError::internal_during("processing the image"))?;
    let properties = process_properties(&ctx.service.property, processed.image_property)
        .map_err(ApiError::internal_during("processing the image properties"))?;

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
        let new_photo = registry
            .new_photo(new_photo)
            .map_err(ApiError::internal_during("registering the photo"))?;
        registry
            .upload_original_image(&photo_id, &original_ext, &bytes)
            .await
            .map_err(ApiError::internal_during("uploading the original image"))?;

        new_photo
    };

    ctx.event_tx
        .send(Event::PhotoRegistered { photo_id: photo_id.clone() })
        .await
        .map_err(ApiError::internal_during("queueing the registration event"))?;

    let response = NewPhotoResponse {
        photo: new_photo.into(),
    };

    Ok((StatusCode::CREATED, Json(success(response))))
}
