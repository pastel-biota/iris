use std::sync::Arc;

use axum::{
    Extension, Json, extract::State, http::StatusCode, response::IntoResponse
};

use crate::{
    Context, federation::{api::IrisHost, protocol}, infra::api::types::{ClientError, SuccessfulResponse, success}
};

/// Get a photo's meta
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    get,
    path = crate::path!(protocol::ListFederatedPhoto),
    params(),
    responses(
        (status = OK, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<protocol::ListFederatedPhotoResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn list(
    State(ctx): State<Arc<Context>>,
    Extension(IrisHost(instance)): Extension<IrisHost>,
) -> impl IntoResponse {
    let photos = ctx.federation.repo
        .list_federated_photos(&instance)
        .unwrap()
        .iter()
        .map(|id| id.to_string())
        .collect();

    (
        StatusCode::OK,
        Json(success(protocol::ListFederatedPhotoResponse { photos })),
    )
        .into_response()
}
