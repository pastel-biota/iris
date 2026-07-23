use std::sync::Arc;

use axum::{
    body::Body, extract::{Path, State}, http::StatusCode, response::IntoResponse
};

use crate::{
    Context, api::{error::ApiError, header::immutable_asset}, auth::{extractor::IrisSession, whitelist}, federation::protocol::Endpoint, infra::api::types::{BinaryBody, ClientError}, model::Identifier
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetImageRequest {
    pub photo_id: Identifier,
    pub image_id: String,
}

pub struct GetImageEndpoint;
impl Endpoint for GetImageEndpoint {
    const PATH: (http::Method, &str) = (http::Method::GET, "/photos/{photo_id}/images/{image_id}");
    type Request = GetImageRequest;
    type Response = ();
}

/// Get actual image
///
/// Retrieves the actual image binary payload.
#[utoipa::path(
    get,
    path = "/{photo_id}/images/{image_id}",
    params(
        ("photo_id" = String, Path),
        ("image_id" = String, Path),
    ),
    responses(
        (
            status = OK,
            description = "The photo/image is found and the image payload is returned.",
            content_type = "application/octet-stream",
            body = BinaryBody,
        ),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_image(
    State(ctx): State<Arc<Context>>,
    IrisSession(session): IrisSession,
    Path((photo_id, image_id)): Path<(Identifier, String)>,
) -> Result<impl IntoResponse, ApiError> {
    whitelist::ensure_photo_allowed(&ctx.auth, &session, &photo_id)
        .await
        .map_err(ApiError::passthrough(ApiError::Forbidden))?;

    let registry = ctx.registry.read().await;
    let photo = registry
        .load_photo(&photo_id)
        .await
        .map_err(ApiError::internal_during("reading the photo metafile"))?
        .ok_or(ApiError::NotFound(
            "the photo with the ID is not found".to_string(),
        ))?;

    let image_meta = photo.images.get(&image_id).ok_or(ApiError::NotFound(
        "the photo was found, but the image with the ID is not found".to_string(),
    ))?;

    let photo_stream = registry
        .load_image(&photo.origin, &image_id, image_meta)
        .await
        .map_err(ApiError::internal_during("reading the image payload"))?;

    let headers = immutable_asset(&image_meta.mime, photo_stream.len)?;

    Ok((
        StatusCode::OK,
        headers,
        Body::from_stream(photo_stream.stream),
    ))
}
