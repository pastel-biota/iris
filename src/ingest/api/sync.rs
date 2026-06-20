use std::sync::Arc;

use axum::{Json, extract::{Path, State}, http::StatusCode, response::IntoResponse};

use crate::{Context, api::error::ApiError, infra::api::types::{ClientError, SuccessfulResponse, success}, model::EntityName};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SyncResponse {
}

/// A new field
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    post,
    // TODO: Replace with the correct path - use {xxx} to accept path parameter
    path = "/sync/{name}",
    params(
        ("name" = EntityName, Path),
    ),
    responses(
        (status = OK, description = "The photo was successfully synchronized", body = SuccessfulResponse<SyncResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn sync(
    State(ctx): State<Arc<Context>>,
    Path((name,)): Path<(EntityName,)>,
) -> Result<impl IntoResponse, ApiError> {
    let mut registry = ctx.registry.write().await;
    registry
        .sync_image_list(&name)
        .await
        .map_err(ApiError::internal_during("synchronizing the image list"))?;

    Ok((StatusCode::OK, Json(success(SyncResponse {}))))
}
