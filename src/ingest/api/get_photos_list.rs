use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use http::Method;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use utoipa::openapi::{RefOr, Schema};

use crate::{
    Context,
    api::error::ApiError,
    auth::extractor::IrisSession,
    federation::protocol::Endpoint,
    infra::api::types::{ClientError, SuccessfulResponse, success},
    ingest::api::scheme::PhotoScheme,
    model::{Identifier, PhotoMeta},
};

const PHOTOS_LIST_CURSOR_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetPhotosListCursor {
    shot_time_unix: i64,
    id: Identifier,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct GetPhotosListCursorPayload {
    version: u8,
    shot_time_unix: i64,
    id: Identifier,
}

impl GetPhotosListCursor {
    fn from_photo(photo: &PhotoMeta) -> Self {
        Self {
            shot_time_unix: photo.shot_time.timestamp(),
            id: photo.id().clone(),
        }
    }

    fn boundary(&self) -> (i64, &Identifier) {
        (self.shot_time_unix, &self.id)
    }
}

impl Serialize for GetPhotosListCursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let payload = GetPhotosListCursorPayload {
            version: PHOTOS_LIST_CURSOR_VERSION,
            shot_time_unix: self.shot_time_unix,
            id: self.id.clone(),
        };
        let payload = serde_json::to_vec(&payload).map_err(serde::ser::Error::custom)?;

        serializer.serialize_str(&BASE64_URL_SAFE_NO_PAD.encode(payload))
    }
}

impl<'de> Deserialize<'de> for GetPhotosListCursor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let payload = BASE64_URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(D::Error::custom)?;
        let payload: GetPhotosListCursorPayload =
            serde_json::from_slice(&payload).map_err(D::Error::custom)?;

        if payload.version != PHOTOS_LIST_CURSOR_VERSION {
            return Err(D::Error::custom(format!(
                "unsupported photos list cursor version: {}",
                payload.version
            )));
        }

        Ok(Self {
            shot_time_unix: payload.shot_time_unix,
            id: payload.id,
        })
    }
}

impl utoipa::PartialSchema for GetPhotosListCursor {
    fn schema() -> RefOr<Schema> {
        utoipa::openapi::Object::builder()
            .schema_type(utoipa::openapi::Type::String)
            .description(Some(
                "An opaque cursor returned by the previous photos list response",
            ))
            .into()
    }
}

impl utoipa::ToSchema for GetPhotosListCursor {
    fn name() -> std::borrow::Cow<'static, str> {
        "GetPhotosListCursor".into()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotosListQuery {
    pub cursor: Option<GetPhotosListCursor>,
    pub size: Option<u32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetPhotosListResponse {
    pub total_count: u32,
    pub next_cursor: Option<GetPhotosListCursor>,
    pub photos: Vec<PhotoScheme>,
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
        ("cursor" = Option<String>, Query, nullable, description = "An opaque cursor returned by the previous response"),
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
    let visibility = session.photo_visibility();

    let registry = ctx.registry.read().await;
    let photos = registry
        .list_images(
            visibility.clone(),
            cursor.as_ref().map(GetPhotosListCursor::boundary),
            size,
        )
        .await
        .map_err(ApiError::internal_during("reading the photo list"))?;

    let total_count = registry
        .total_count(visibility)
        .await
        .map_err(ApiError::internal_during("reading the total count"))?;

    let next_cursor = if photos.len() == size {
        photos.last().map(GetPhotosListCursor::from_photo)
    } else {
        None
    };

    Ok((
        StatusCode::OK,
        Json(success(GetPhotosListResponse {
            total_count,
            next_cursor,
            photos: photos.into_iter().map(Into::into).collect(),
        })),
    ))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn cursor_round_trips_as_an_opaque_string() {
        let cursor = GetPhotosListCursor {
            shot_time_unix: 1_768_111_035,
            id: Identifier::from_str("202601-01KY7C9TY66CRXTR78207RWH7J").unwrap(),
        };

        let encoded = serde_json::to_value(&cursor).unwrap();

        assert!(encoded.is_string());
        assert_eq!(
            serde_json::from_value::<GetPhotosListCursor>(encoded).unwrap(),
            cursor
        );
    }

    #[test]
    fn cursor_rejects_an_unsupported_payload_version() {
        let payload = serde_json::json!({
            "version": 2,
            "shot_time_unix": 1_768_111_035,
            "id": "202601-01KY7C9TY66CRXTR78207RWH7J",
        });
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());

        let error =
            serde_json::from_value::<GetPhotosListCursor>(serde_json::json!(encoded)).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unsupported photos list cursor version: 2")
        );
    }
}
