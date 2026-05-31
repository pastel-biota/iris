use std::sync::Arc;

use axum::{
    Json, body::Body, extract::{Path, State}, http::{StatusCode, header}, response::IntoResponse
};
use http::{HeaderMap, HeaderValue};

use crate::{
    Context, auth::{extractor::IrisSession, whitelist}, federation::protocol::Endpoint, infra::api::types::{BinaryBody, ClientError, client_error}, model::Identifier
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct GetImageRequest {
    pub photo_id: Identifier,
    pub image_id: String,
}

pub struct GetImageEndpoint;
impl Endpoint for GetImageEndpoint {
    const PATH: (http::Method, &str) = (http::Method::GET, "/photos/{photo_id}/images/{image_id}");
    type Request = GetImageRequest;
    type Response = ();
}

/// Get actual image
///
/// Retrieves the actual image binary payload.
#[utoipa::path(
    get,
    path = "/{photo_id}/images/{image_id}",
    params(
        ("photo_id" = String, Path),
        ("image_id" = String, Path),
    ),
    responses(
        (
            status = OK,
            description = "The photo/image is found and the image payload is returned.",
            content_type = "application/octet-stream",
            body = BinaryBody,
        ),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn get_image(
    State(ctx): State<Arc<Context>>,
    IrisSession(session): IrisSession,
    Path((photo_id, image_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let Ok(photo_id) = photo_id.parse::<Identifier>() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(client_error("Photo id is not valid as the Id")),
        )
            .into_response();
    };

    if let Err(err) = whitelist::ensure_photo_allowed(&ctx.auth, &session, &photo_id) {
        return (StatusCode::FORBIDDEN, Json(client_error(&err.to_string()))).into_response();
    }

    let mut registry = ctx.registry.write().await;
    let photo = match registry.load_photo(&photo_id).await {
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

    let Some(image_meta) = photo.images.get(&image_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(client_error(
                "the photo was found, but the image with the ID is not found",
            )),
        )
            .into_response();
    };

    let photo_stream = match registry.load_image(&photo_id, &image_id, image_meta).await {
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

    let mut headers = HeaderMap::<HeaderValue>::from_iter([
        (header::CONTENT_TYPE, image_meta.mime.as_str().try_into().unwrap()),
        (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=2592000, immutable")),
    ]);

    if let Some(len) = photo_stream.len {
        headers.insert(
            header::CONTENT_LENGTH,
            (&len.to_string()).try_into().unwrap(),
        );
    }

    (
        StatusCode::OK,
        headers,
        Body::from_stream(photo_stream.stream),
    )
        .into_response()
}
