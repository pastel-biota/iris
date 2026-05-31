use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http::Method;

use crate::{
    Context, auth::{extractor::IrisSession, whitelist::{self, PagedIdentifiers}}, federation::protocol::Endpoint, infra::api::types::{
        ClientError, SuccessfulResponse, client_error, success,
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
) -> impl IntoResponse {
    let cursor = query.cursor;
    let size = query.size.unwrap_or(50).try_into().unwrap();

    let photo_ids = match whitelist::get_allowed_photos(&ctx.auth, &session, size, cursor.clone()) {
        Ok(ids) => ids,
        Err(err) => {
            return (StatusCode::FORBIDDEN, Json(client_error(&err.to_string()))).into_response();
        }
    };

    let list_result = if let Some(ids) = photo_ids {
        retrieve_from_provided_list(ctx, ids).await
    } else {
        retrieve_from_whole_set(ctx.clone(), cursor.clone(), size).await
    };

    match list_result {
        Ok(list) => (StatusCode::OK, Json(success(list))).into_response(),
        Err(err) => err.into_response(),
    }
}

async fn retrieve_from_whole_set(
    ctx: Arc<Context>,
    cursor: Option<Identifier>,
    size: usize,
) -> Result<GetPhotosListResponse, Response> {
    let mut registry = ctx.registry.write().await;
    let photos = match registry.list_images(cursor.as_ref(), size) {
        Ok(photo) => photo,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(client_error(&format!(
                    "there was an internal error during read the photo: {:#?}",
                    err
                ))),
            )
                .into_response());
        }
    };

    let total_count = match registry.total_count() {
        Ok(total_count) => total_count,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(client_error(&format!(
                    "there was an internal error during read the photo: {:#?}",
                    err
                ))),
            )
                .into_response());
        }
    };

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
) -> Result<GetPhotosListResponse, Response> {
    let mut registry = ctx.registry.write().await;

    let photos = match registry.get_photos_list_by_id_list(&ids.ids) {
        Ok(photo) => photo,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(client_error(&format!(
                    "there was an internal error during read the photo: {:#?}",
                    err
                ))),
            )
                .into_response());
        }
    };

    Ok(GetPhotosListResponse {
        total_count: ids.total_count,
        next_cursor: ids.next_cursor,
        photos: photos.into_iter().cloned().map(Into::into).collect(),
    })
}
