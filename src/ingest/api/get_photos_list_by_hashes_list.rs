use std::{collections::HashMap, sync::Arc};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::{
    Context, api::error::ApiError, auth::extractor::ValidUserSession, infra::api::types::{
        ClientError, SuccessfulResponse, success,
    }, ingest::api::scheme::PhotoReferenceSchema
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotosListByHashesListParam {
    /// Hashes to look up.
    #[schema(
        example = json!( [
            "2653cb6500059d0316f40d0d24b7a11ace792a03eeab5b5e183275443e230612",
            "cb9e7fd9ead80d775c9cbe0fe5202a1f3202bcde6a9da92e54bc57a6e7b8931e",
        ]),
        min_items = 1,
        max_items = 100
    )]
    hashes: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct GetPhotosListByHashesListResponse<'a> {
    /// The map of found hashes and corresponding photos.
    /// Note that this map does not necessarily include all hashes specified,
    /// as not found hashes will not be included.
    photos: HashMap<&'a str, PhotoReferenceSchema>,
}

/// Get photos list by hashes list
///
/// Retrieves the list of photos from the list of hashes.
/// You need to be logged in to use this endpoint.
#[utoipa::path(
    post,
    path = "/by-hashes",
    security(
        ("session_header" = []),
        ("session_cookie" = [])
    ),
    request_body(content_type = "application/json", content = GetPhotosListByHashesListParam),
    responses(
        (status = OK, description = "Found photos. If no photo is found, the endpoint will return 200 OK with an empty array.", body = SuccessfulResponse<GetPhotosListByHashesListResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_photos_list_by_hashes_list(
    State(ctx): State<Arc<Context>>,
    ValidUserSession(_): ValidUserSession,
    Json(param): Json<GetPhotosListByHashesListParam>,
) -> Result<impl IntoResponse, ApiError> {
    let mut registry = ctx.registry.write().await;

    let photos = registry
        .get_photos_list_by_hashes_list(param.hashes.as_slice())
        .map_err(ApiError::internal_during("searching photos from hashes"))?;

    let photos = photos
        .into_iter()
        .map(|(k, v)| (k, v.clone().into()))
        .collect::<HashMap<_, _>>();

    let response = GetPhotosListByHashesListResponse { photos };

    Ok((StatusCode::OK, Json(success(response))).into_response())
}
