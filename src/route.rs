pub mod new_photo;
pub mod upload_image;
pub mod get_photo_meta;
pub mod scheme;
pub mod get_image;

use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::Context;

pub fn photo_route(ctx: Arc<Context>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(new_photo::new_photo))
        .routes(routes!(upload_image::upload_image))
        .routes(routes!(get_photo_meta::get_photo_meta))
        .routes(routes!(get_image::get_image))
        .with_state(ctx)
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SuccessfulResponse<T> {
    #[schema(example = "okay")]
    status: &'static str,

    response: T,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ClientError {
    #[schema(example = "error")]
    status: &'static str,

    #[schema(example = "The value was invalid...")]
    reason: String,
}

pub fn success<T>(reason: T) -> SuccessfulResponse<T> {
    SuccessfulResponse {
        status: "okay",
        response: reason,
    }
}

pub fn client_error(reason: &str) -> ClientError {
    ClientError {
        status: "error",
        reason: reason.to_string()
    }
}

