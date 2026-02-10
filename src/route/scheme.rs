use crate::{
    infra::meta::{ImageMeta, PhotoMeta},
    model::Properties,
};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PhotoScheme {
    /// Identifier assigned to the created photo.
    #[schema(example = "202601_img_0001_jpg-01AAAA")]
    id: String,

    /// The list of identifiers assigned to the specified images.
    /// The image ID is used to upload the actual image later.
    images: Vec<PhotoImages>,

    properties: PropertiesSchema,
}

impl From<PhotoMeta> for PhotoScheme {
    fn from(value: PhotoMeta) -> Self {
        Self {
            id: value.id.to_string(),
            images: value.images.into_iter().map(Into::into).collect(),
            properties: value.properties.into(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PhotoImages {
    #[schema(example = "01AAAA")]
    image_id: String,

    #[schema(example = "1080p")]
    name: String,

    #[schema(example = "jpg")]
    ext: String,

    #[schema(example = 1920)]
    width: u32,

    #[schema(example = 1080)]
    height: u32,
}

impl From<ImageMeta> for PhotoImages {
    fn from(value: ImageMeta) -> Self {
        Self {
            name: value.name,
            width: value.width,
            height: value.height,
            image_id: value.image_id,
            ext: value.extension,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
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

impl From<Properties> for PropertiesSchema {
    fn from(value: Properties) -> Self {
        Self {
            machine: value.machine,
            lens: value.lens,
            gps_lng_lat: value.gps_lng_lat.map(|(lng, lat)| vec![lng, lat]),
        }
    }
}
