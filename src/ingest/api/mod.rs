pub mod delete;
pub mod get_image;
pub mod get_photos_list;
pub mod get_photo_meta;
pub mod get_photos_list_by_hashes_list;
pub mod new_photo;
pub mod reprocess;
pub mod sync;
pub mod scheme;

use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::Context;

pub fn photo_route(_ctx: Arc<Context>) -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(delete::delete))
        .routes(routes!(new_photo::new_photo))
        .routes(routes!(get_photo_meta::get_photo_meta))
        .routes(routes!(get_image::get_image))
        .routes(routes!(get_photos_list::get_photos_list))
        .routes(routes!(
            get_photos_list_by_hashes_list::get_photos_list_by_hashes_list
        ))
        .routes(routes!(reprocess::reprocess))
        .routes(routes!(sync::sync))
}

