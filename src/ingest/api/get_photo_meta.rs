use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context,
    ingest::api::scheme::PhotoScheme,
    infra::api::types::{ClientError, SuccessfulResponse, client_error, success},
    model::Identifier,
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct GetPhotoMetaResponse {
    photo: PhotoScheme,
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
    Path((photo_id,)): Path<(String,)>,
) -> impl IntoResponse {
    let Ok(photo_id) = photo_id.parse::<Identifier>() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(client_error("Photo id is not valid as the Id")),
        )
            .into_response();
    };

    let mut registry = ctx.registry.write().await;
    let photo = match registry.load_photo(&photo_id) {
        Ok(photo) => photo,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(client_error(&format!(
                    "there was an internal error during reading the photo metafile: {:#?}",
                    err
                ))),
            )
                .into_response();
        }
    };

    let Some(photo) = photo else {
        return (
            StatusCode::NOT_FOUND,
            Json(client_error("the photo with the ID is not found")),
        )
            .into_response();
    };
    let photo = PhotoScheme::from(photo);

    (
        StatusCode::OK,
        Json(success(GetPhotoMetaResponse { photo })),
    )
        .into_response()
}
