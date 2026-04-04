use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    Context, federation::protocol, infra::api::types::{ClientError, SuccessfulResponse, success}
};

/// Get a photo's meta
///
/// This is a new field. This initially returns implemented error.
#[utoipa::path(
    get,
    path = crate::path!(protocol::Health),
    params(),
    responses(
        (status = OK, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<protocol::HealthResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn ping(
    State(ctx): State<Arc<Context>>,
) -> impl IntoResponse {
    for (key, host) in ctx.federation.config.hosts.iter() {
        if &ctx.base.host == key {
            continue;
        }

        let req = protocol::request::<protocol::Health>(
            ctx.clone(),
            &host.origin,
            ()
        ).await;
        dbg!(req).unwrap();
    }

    (
        StatusCode::OK,
        Json(success(protocol::HealthResponse {})),
    )
        .into_response()
}
