use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use http::Method;

use crate::{
    Context, api::error::ApiError, auth::{extractor::IrisSession, whitelist::{self, PagedIdentifiers}}, federation::protocol::Endpoint, infra::api::types::{
        ClientError, SuccessfulResponse, success,
    }, ingest::api::scheme::PhotoReferenceSchema, model::Identifier
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotosListQuery {
    pub cursor: Option<Identifier>,
    pub size: Option<u32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotosListResponse {
    pub total_count: u32,
    pub next_cursor: Option<Identifier>,
    pub photos: Vec<PhotoReferenceSchema>,
}

pub struct GetPhotosListEndpoint;
impl Endpoint for GetPhotosListEndpoint {
    const PATH: (Method, &str) = (Method::GET, "/photos");
    type Request = GetPhotosListQuery;
    type Response = GetPhotosListResponse;
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
        (status = OK, description = "The list of images.", body = SuccessfulResponse<GetPhotosListResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_photos_list(
    State(ctx): State<Arc<Context>>,
    IrisSession(session): IrisSession,
    Query(query): Query<GetPhotosListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let cursor = query.cursor;
    let size = query.size.unwrap_or(50) as usize;

    let photo_ids = whitelist::get_allowed_photos(&ctx.auth, &session, size, cursor.clone())
        .map_err(ApiError::passthrough(ApiError::Forbidden))?;

    let list = if let Some(ids) = photo_ids {
        retrieve_from_provided_list(ctx, ids).await?
    } else {
        retrieve_from_whole_set(ctx.clone(), cursor.clone(), size).await?
    };

    Ok((StatusCode::OK, Json(success(list))))
}

async fn retrieve_from_whole_set(
    ctx: Arc<Context>,
    cursor: Option<Identifier>,
    size: usize,
) -> Result<GetPhotosListResponse, ApiError> {
    let mut registry = ctx.registry.write().await;
    let photos = registry
        .list_images(cursor.as_ref(), size)
        .map_err(ApiError::internal_during("reading the photo list"))?;

    let total_count = registry
        .total_count()
        .map_err(ApiError::internal_during("reading the total count"))?;

    let next_cursor = if photos.len() == size {
        photos.last().map(|photo| photo.id())
    } else {
        None
    };

    Ok(GetPhotosListResponse {
        total_count,
        next_cursor: next_cursor.cloned(),
        photos: photos.into_iter().map(Into::into).collect(),
    })
}

async fn retrieve_from_provided_list(
    ctx: Arc<Context>,
    ids: PagedIdentifiers,
) -> Result<GetPhotosListResponse, ApiError> {
    let mut registry = ctx.registry.write().await;

    let photos = registry
        .get_photos_list_by_id_list(&ids.ids)
        .map_err(ApiError::internal_during("reading the photo list"))?;

    Ok(GetPhotosListResponse {
        total_count: ids.total_count,
        next_cursor: ids.next_cursor,
        photos: photos.into_iter().cloned().map(Into::into).collect(),
    })
}
