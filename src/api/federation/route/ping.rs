use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context,
    api::types::{ClientError, SuccessfulResponse, success},
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PingResponse {
}

/// Get a photo's meta
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    get,
    path = "/ping",
    params(),
    responses(
        (status = OK, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<PingResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn ping(
    State(ctx): State<Arc<Context>>,
) -> impl IntoResponse {
    dbg!(&ctx.federation.config);

    (
        StatusCode::OK,
        Json(success(PingResponse {})),
    )
        .into_response()
}
