use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context,
    api::error::ApiError,
    auth::{extractor::IrisSession, whitelist},
    federation::protocol::Endpoint,
    infra::api::types::{ClientError, SuccessfulResponse, success},
    ingest::api::scheme::PhotoScheme,
    model::Identifier,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotoMetaRequest {
    pub photo_id: Identifier,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotoMetaResponse {
    pub photo: PhotoScheme,
}

pub struct GetPhotoMetaEndpoint;
impl Endpoint for GetPhotoMetaEndpoint {
    const PATH: (http::Method, &str) = (http::Method::GET, "/photos/{photo_id}");
    type Request = GetPhotoMetaRequest;
    type Response = GetPhotoMetaResponse;
}

/// Get a photo's meta
#[utoipa::path(
    get,
    path = "/{photo_id}",
    params(
        ("photo_id" = String, Path),
    ),
    responses(
        (status = OK, description = "A found photo's metadata information.", body = SuccessfulResponse<GetPhotoMetaResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_photo_meta(
    State(ctx): State<Arc<Context>>,
    IrisSession(session): IrisSession,
    Path((photo_id,)): Path<(Identifier,)>,
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

    let photo = PhotoScheme::from(photo);

    Ok((
        StatusCode::OK,
        Json(success(GetPhotoMetaResponse { photo })),
    )
        .into_response())
}
