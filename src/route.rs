pub mod get_image;
pub mod get_images_list;
pub mod get_photo_meta;
pub mod new_photo;
pub mod scheme;
pub mod upload_image;
pub mod get_photos_list_by_hashes_list;

use std::{ops::Deref, sync::Arc};

use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::Context;

pub fn photo_route(ctx: Arc<Context>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(new_photo::new_photo))
        .routes(routes!(upload_image::upload_image))
        .routes(routes!(get_photo_meta::get_photo_meta))
        .routes(routes!(get_image::get_image))
        .routes(routes!(get_images_list::get_images_list))
        .routes(routes!(get_photos_list_by_hashes_list::get_photos_list_by_hashes_list))
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
        reason: reason.to_string(),
    }
}

#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
struct BinaryBody(Vec<u8>);

impl Deref for BinaryBody {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<u8>> for BinaryBody {
    fn from(value: Vec<u8>) -> Self {
        BinaryBody(value)
    }
}
