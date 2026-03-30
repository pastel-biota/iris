mod get_image;
mod get_images_list;
mod get_photo_meta;
mod get_photos_list_by_hashes_list;
mod new_photo;
mod reprocess;
mod scheme;

use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::Context;

pub fn photo_route(ctx: Arc<Context>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(new_photo::new_photo))
        .routes(routes!(get_photo_meta::get_photo_meta))
        .routes(routes!(get_image::get_image))
        .routes(routes!(get_images_list::get_images_list))
        .routes(routes!(
            get_photos_list_by_hashes_list::get_photos_list_by_hashes_list
        ))
        .routes(routes!(reprocess::reprocess))
        .with_state(ctx)
}

