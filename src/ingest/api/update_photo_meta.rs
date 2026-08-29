use std::sync::Arc;

use crate::{
    Context,
    api::error::ApiError,
    auth::{extractor::ValidUserSession, whitelist},
    infra::api::types::{ClientError, SuccessfulResponse, success},
    ingest::api::scheme::PhotoScheme,
    model::{
        PhotoMeta,
        service::photo::{ChangeValidationError, try_apply_update},
    },
};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PhotoMetaModifyResponse {
    photo: PhotoScheme,
}

/// Modify the photo's metadata by replacing it with the provided JSON
///
/// The provided JSON payload will be first parsed into the internal representation of the photo
/// metadata, and then save to the replacement. Due to this nature of the process the result photo
/// metadata might not exactly match to the provided JSON payload.
#[utoipa::path(
    put,
    path = "/meta",
    request_body(content = PhotoScheme, content_type = "application/json"),
    security(
        ("session_header" = []),
        ("session_cookie" = [])
    ),
    responses(
        (status = OK, description = "The photo was modified successfully", body = SuccessfulResponse<PhotoMetaModifyResponse>),
        (status = BAD_REQUEST, description = "The provided photo meta JSON cannot be used for the update", body = ClientError),
        (status = NO_CONTENT, description = "All defined images were already creted"),
    )
)]
pub async fn update_photo_meta(
    State(ctx): State<Arc<Context>>,
    ValidUserSession(session): ValidUserSession,
    Json(payload): Json<PhotoScheme>,
) -> Result<impl IntoResponse, ApiError> {
    let new_photo = PhotoMeta::from(payload);
    let photo_id = new_photo.origin.local_id()
        .ok_or(ApiError::BadRequest("Only the local photo's metadata can be updated. Consider modifying at the owner and syncing the photos".into()))?
        .clone();

    whitelist::ensure_photo_allowed(&ctx.auth, &session.clone().into(), &&*photo_id)
        .await
        .map_err(ApiError::passthrough(ApiError::Forbidden))?;

    let photo = {
        let registry = ctx.registry.read().await;
        registry
            .load_photo(&photo_id)
            .await
            .map_err(ApiError::internal_during("reading the photo metafile"))?
            .ok_or(ApiError::NotFound(
                "the photo with the ID is not found".to_string(),
            ))?
    };

    let new_photo = match try_apply_update(&photo, new_photo) {
        Ok(new_photo) => new_photo,
        Err(ChangeValidationError::Rejected(rejections)) => {
            let rejection_msg = rejections
                .iter()
                .map(|rej| format!("- {rej}\n"))
                .collect::<String>();

            return Err(ApiError::BadRequest(format!(
                "The provided photo meta JSON cannot be used for the update:\n{}",
                rejection_msg
            )));
        }
        Err(ChangeValidationError::Anyhow(error)) => {
            return Err(ApiError::internal(error));
        }
    };

    let photo = {
        let mut registry = ctx.registry.write().await;
        registry
            .update_photo_meta(new_photo.clone())
            .map_err(ApiError::internal_during("updating the photo metafile"))?;

        registry
            .load_photo(&photo_id)
            .await
            .map_err(ApiError::internal_during("updating the photo metafile"))?
            .ok_or(ApiError::InternalError(
                "The photo disappared after updating the metadata file".to_string(),
            ))?
    };

    Ok((
        StatusCode::OK,
        Json(success(PhotoMetaModifyResponse {
            photo: photo.into(),
        })),
    ))
}
