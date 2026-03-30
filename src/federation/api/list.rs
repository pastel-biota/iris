use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context, federation::extractor::RequestingInstance, infra::api::types::{ClientError, SuccessfulResponse, success}, model::Identifier
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ListFederatedPhotoResponse {
    photos: Vec<String>,
}

/// Get a photo's meta
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    get,
    path = "/photos",
    params(),
    responses(
        (status = OK, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<ListFederatedPhotoResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn list(
    State(ctx): State<Arc<Context>>,
    RequestingInstance(instance): RequestingInstance,
) -> impl IntoResponse {
    dbg!(&ctx.federation.config);
    dbg!(&instance);

    let photos = ctx.federation.repo
        .list_federated_photos("local-2")
        .unwrap()
        .iter()
        .map(|id| id.to_string())
        .collect();

    (
        StatusCode::OK,
        Json(success(ListFederatedPhotoResponse { photos })),
    )
        .into_response()
}
