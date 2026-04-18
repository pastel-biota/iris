use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context,
    ingest::api::scheme::PhotoReferenceSchema,
    infra::api::types::{
        ClientError, SuccessfulResponse, client_error, success,
    },
    model::Identifier,
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct GetImagesListQuery {
    cursor: Option<String>,
    size: Option<u32>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
struct GetImagesListResponse {
    total_count: u32,
    next_cursor: Option<String>,
    photos: Vec<PhotoReferenceSchema>,
}

/// Get registered photos' list
#[utoipa::path(
    get,
    path = "/",
    params(
        ("cursor" = Option<String>, Query, nullable, description = "The pagination cursor - retrieves from beginning"),
        ("size" = Option<u32>, Query, nullable, description = "the default is 50"),
    ),
    responses(
        (status = OK, description = "The list of images.", body = SuccessfulResponse<GetImagesListResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_images_list(
    State(ctx): State<Arc<Context>>,
    Query(query): Query<GetImagesListQuery>,
) -> impl IntoResponse {
    let cursor = query
        .cursor
        .map(|cursor| cursor.parse::<Identifier>())
        .transpose()
        .unwrap();
    let size = query.size.unwrap_or(50).try_into().unwrap();

    let mut registry = ctx.registry.write().await;
    let photos = match registry.list_images(cursor.as_ref(), size) {
        Ok(photo) => photo,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(client_error(&format!(
                    "there was an internal error during read the photo: {:#?}",
                    err
                ))),
            )
                .into_response();
        }
    };

    let total_count = match registry.total_count() {
        Ok(total_count) => total_count,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(client_error(&format!(
                    "there was an internal error during read the photo: {:#?}",
                    err
                ))),
            )
                .into_response();
        }
    };

    let next_cursor = if photos.len() == size {
        photos.last().map(|photo| photo.id())
    } else {
        None
    };

    let photo = GetImagesListResponse {
        total_count,
        next_cursor: next_cursor.map(|cursor| cursor.to_string()),
        photos: photos.into_iter().map(Into::into).collect(),
    };

    (StatusCode::OK, Json(success(photo))).into_response()
}
