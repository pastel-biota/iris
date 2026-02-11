use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, FixedOffset};

use crate::{
    Context,
    model::Identifier,
    model::{ImageMeta, PhotoMeta, Properties},
    route::{ClientError, SuccessfulResponse, client_error, success},
};

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct NewPhotoParam {
    /// The datetime the photo was taken.
    /// The shot timstamp is used for the part of identification and indexing.
    #[schema(example = "2026-01-02T03:45:06+09:00")]
    shot_date: String,

    /// The file name of the original picture.
    /// This is also used for the part of identification, but any arbitary string can be specified.
    #[schema(example = "IMG_0001.JPG")]
    file_name: String,

    /// The hexadecimal representation of SHA256 hash.
    #[schema(
        example = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        min_length = 64,
        max_length = 64,
    )]
    original_sha256: String,

    /// The list of dimensions of the images to be uploaded later.
    /// Each images will get each image ID assigned for the later upload.
    uploading_images: Vec<NewImages>,

    properties: PropertiesSchema,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct NewImages {
    #[schema(example = "1080p")]
    name: String,

    #[schema(example = "jpg")]
    ext: String,

    #[schema(example = 1920)]
    width: u32,

    #[schema(example = 1080)]
    height: u32,
}

#[derive(Clone, Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct PropertiesSchema {
    #[schema(example = "X-T4")]
    pub machine: String,

    #[schema(example = "SIGMA")]
    pub lens: String,

    #[schema(
        example = json!([36.123456, 138.123456]),
        min_items = 2,
        max_items = 2,
    )]
    // Can't do Option<(f32, f32)> here because it results to
    // OpenAPI 3.0 Incompatible scheme!
    pub gps_lng_lat: Option<Vec<f32>>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NewPhotoResponse {
    /// Identifier assigned to the created photo.
    #[schema(example = "202601_img_0001_jpg-01AAAA")]
    id: String,

    /// The list of identifiers assigned to the specified images.
    /// The image ID is used to upload the actual image later.
    images: Vec<NewPhotoImageResponse>,

    /// How much of parallel upload is accepted for the upload of this image.
    max_parallelism: u8,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NewPhotoImageResponse {
    #[schema(example = "1080p")]
    name: String,

    #[schema(example = "01AAAA")]
    image_id: String,
}

/// Registers a new photo
///
/// Register a new photo, and prepare for the upload for the actual image.
#[utoipa::path(
    post,
    path = "/",
    request_body(content_type = "application/json", content = NewPhotoParam),
    responses(
        (status = CREATED, description = "The photo was registered and ready for image upload.", body = SuccessfulResponse<NewPhotoResponse>),
        (status = BAD_REQUEST, description = "The parameter/body was invalid", body = ClientError),
    )
)]
pub async fn new_photo(
    State(ctx): State<Arc<Context>>,
    Json(param): Json<NewPhotoParam>,
) -> impl IntoResponse {
    let shot_date = match DateTime::<FixedOffset>::parse_from_rfc3339(&param.shot_date) {
        Ok(date) => date,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(client_error(&format!("shot_date was invalid: {}", err))),
            )
                .into_response();
        }
    };

    if param.uploading_images.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(client_error("There must be at least one image")),
        )
            .into_response();
    }

    let id = Identifier::new(&shot_date, &param.file_name, &ulid::Ulid::new().to_string());

    let images: Vec<_> = param
        .uploading_images
        .into_iter()
        .map(|img| ImageMeta {
            width: img.width,
            height: img.height,
            name: img.name,
            extension: img.ext,
            image_id: ulid::Ulid::new().to_string(),
        })
        .collect();

    let gps_lng_lat = match param.properties.gps_lng_lat.as_deref() {
        None => None,
        Some([lng, lat]) => Some((*lng, *lat)),
        Some(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(client_error("properties.gps_lng_lat is not in the right form: expected two element array/tuple"))
            )
                .into_response();
        }
    };

    let mut registry = ctx.registry.write().await;
    registry
        .new_photo(PhotoMeta {
            id: id.clone(),
            images: images.clone(),
            original_sha256: param.original_sha256,
            shot_time: shot_date,
            properties: Properties {
                machine: param.properties.machine,
                lens: param.properties.lens,
                gps_lng_lat,
            },
        })
        .unwrap();

    let response = NewPhotoResponse {
        id: id.to_string(),
        images: images
            .into_iter()
            .map(|image| NewPhotoImageResponse {
                name: image.name,
                image_id: image.image_id,
            })
            .collect(),
        max_parallelism: 4,
    };

    (StatusCode::CREATED, Json(success(response))).into_response()
}
