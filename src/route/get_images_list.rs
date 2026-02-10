use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context,
    route::{ClientError, SuccessfulResponse, client_error, success},
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct GetImagesListQuery {
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
struct GetImagesListResponse {
    total_count: u32,
    photos: Vec<PhotoSchema>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
struct PhotoSchema {
    id: String,
    year: i32,
    month: u32,
    images: Vec<ImageSchema>,
    shot_time: String,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
struct ImageSchema {
    id: String,
    height: u32,
    ext: String,
}

/// Get the list of images.
#[utoipa::path(
    get,
    path = "/",
    params(
        ("offset" = Option<usize>, Query, nullable, description = "the default is 0"),
        ("limit" = Option<usize>, Query, nullable, description = "the default is 200"),
    ),
    responses(
        (status = OK, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<GetImagesListResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_images_list(
    State(ctx): State<Arc<Context>>,
    Query(query): Query<GetImagesListQuery>,
) -> impl IntoResponse {
    let offset = query.offset.unwrap_or(0).try_into().unwrap();
    let limit = query.limit.unwrap_or(200).try_into().unwrap();

    let mut registry = ctx.registry.write().await;
    let photos = match registry.list_images(offset, limit) {
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

    let photo = GetImagesListResponse {
        total_count,
        photos: photos
            .into_iter()
            .map(|photo| PhotoSchema {
                year: photo.id.year,
                month: photo.id.month,
                id: photo.id.to_string(),
                shot_time: photo.shot_time.to_rfc3339(),
                images: photo
                    .images
                    .into_iter()
                    .map(|img| ImageSchema {
                        id: img.id,
                        height: img.height,
                        ext: img.ext,
                    })
                    .collect(),
            })
            .collect(),
    };

    (StatusCode::OK, Json(success(photo))).into_response()
}
