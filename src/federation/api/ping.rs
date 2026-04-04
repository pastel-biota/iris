use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context, federation::{api::list::ListFederatedPhotoResponse, request::create_client}, infra::api::types::{ClientError, IrisResponse, SuccessfulResponse, success}
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
    let req = create_client()
        .get("http://home-prime.akita-koi.ts.net:8080/federation/photos?limit=1&offset=10")
        .with_extension(ctx.clone())
        .send()
        .await
        .unwrap()
        .json::<IrisResponse<ListFederatedPhotoResponse>>()
        .await
        .unwrap();

    dbg!(req);

    (
        StatusCode::OK,
        Json(success(PingResponse {})),
    )
        .into_response()
}
