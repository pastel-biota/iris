use std::collections::HashMap;

use crate::ingest::{
    infra::photo_index::PhotoReference,
    model::{ImageMeta, PhotoMeta, Properties},
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PhotoScheme {
    /// Identifier assigned to the created photo.
    #[schema(example = "202601_img_0001_jpg-01AAAA")]
    id: String,

    /// The hexadecimal representation of SHA256 hash.
    #[schema(
        example = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        min_length = 64,
        max_length = 64
    )]
    original_sha256: String,

    /// The list of identifiers assigned to the specified images.
    /// The image ID is used to upload the actual image later.
    images: HashMap<String, ImageMetaScheme>,

    shot_datetime: String,

    #[schema(example = "#123456")]
    representative_color: String,

    properties: PropertiesSchema,
}

impl From<PhotoMeta> for PhotoScheme {
    fn from(value: PhotoMeta) -> Self {
        Self {
            id: value.id.to_string(),
            original_sha256: value.original_sha256,
            images: value
                .images
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            properties: value.properties.into(),
            shot_datetime: value.shot_time.to_rfc3339(),
            representative_color: {
                let [r, g, b] = value.representative_rgb;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            },
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ImageMetaScheme {
    #[schema(example = "jpg")]
    ext: String,

    #[schema(example = "image/jpeg")]
    mime: String,

    #[schema(example = 1920)]
    width: u32,

    #[schema(example = 1080)]
    height: u32,
}

impl From<ImageMeta> for ImageMetaScheme {
    fn from(value: ImageMeta) -> Self {
        Self {
            width: value.width,
            height: value.height,
            ext: value.extension,
            mime: value.mime,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PropertiesSchema {
    #[schema(example = "X-T4")]
    pub machine: String,

    #[schema(example = "SIGMA")]
    pub lens: Option<String>,

    #[schema(
        example = json!([36.123456, 138.123456]),
        min_items = 2,
        max_items = 2,
    )]
    // Can't do Option<(f32, f32)> here because it results to
    // OpenAPI 3.0 Incompatible scheme!
    pub gps_lat_lng: Option<Vec<f64>>,

    #[schema(example = 5.4)]
    pub f_number: Option<f64>,

    #[schema(example = 400)]
    pub shutter_speed: Option<f32>,

    #[schema(example = true)]
    pub shutter_speed_controlled: Option<bool>,

    #[schema(example = 160)]
    pub iso: Option<u64>,

    #[schema(example = 50.0)]
    pub focal: Option<f64>,
}

impl From<Properties> for PropertiesSchema {
    fn from(value: Properties) -> Self {
        Self {
            machine: value.machine,
            lens: value.lens,
            gps_lat_lng: value.gps_lat_lng.map(|(lng, lat)| vec![lng, lat]),
            f_number: value.f_number,
            shutter_speed: value.shutter_speed.map(|speed| speed.0),
            shutter_speed_controlled: value.shutter_speed_controlled,
            iso: value.iso,
            focal: value.focal,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PhotoReferenceSchema {
    id: String,
    year: i32,
    month: u32,
    original_sha256: String,
    images: HashMap<String, ImageMetaScheme>,
    shot_time: String,
    representative_color: String,
}

impl From<PhotoReference> for PhotoReferenceSchema {
    fn from(value: PhotoReference) -> Self {
        PhotoReferenceSchema {
            year: value.id.year,
            month: value.id.month,
            id: value.id.to_string(),
            original_sha256: value.hash,
            shot_time: value.shot_time.to_rfc3339(),
            images: value
                .images
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            representative_color: {
                let [r, g, b] = value.representative_rgb;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            },
        }
    }
}
