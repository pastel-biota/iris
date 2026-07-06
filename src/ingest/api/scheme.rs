use std::collections::HashMap;

use crate::model::{
    EntityName, Identifier, ImageMeta, LocalIdentifier, NormalizedRational, PhotoMeta, PhotoOrigin,
    PhotoReference, Properties, RemoteOrigin,
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PhotoScheme {
    /// Identifier assigned to the created photo.
    #[schema(example = "202601_img_0001_jpg-01AAAA")]
    id: Identifier,

    federator: Option<EntityName>,

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

    tags: HashMap<String, Vec<String>>,

    shot_datetime: String,

    #[schema(example = "#123456")]
    representative_color: String,

    properties: PropertiesSchema,
}

impl From<PhotoMeta> for PhotoScheme {
    fn from(value: PhotoMeta) -> Self {
        Self {
            id: value.id().clone(),
            federator: value.origin.federator().cloned(),
            original_sha256: value.original_sha256,
            images: value
                .images
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            tags: value.tags,
            properties: value.properties.into(),
            shot_datetime: value.shot_time.to_rfc3339(),
            representative_color: {
                let [r, g, b] = value.representative_rgb;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            },
        }
    }
}

impl From<PhotoScheme> for PhotoMeta {
    fn from(value: PhotoScheme) -> Self {
        let origin = if let Some(federator) = value.federator {
            PhotoOrigin::Federated(RemoteOrigin {
                federator,
                identifier: value.id,
            })
        } else {
            PhotoOrigin::Local(LocalIdentifier(value.id))
        };

        PhotoMeta {
            origin,
            shot_time: chrono::DateTime::parse_from_rfc3339(&value.shot_datetime).unwrap(),
            images: value
                .images
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            tags: value.tags,
            representative_rgb: {
                let value: u64 = u64::from_str_radix(&value.representative_color[1..], 16).unwrap();
                [
                    ((value & 0xFF0000) >> 16) as u8,
                    ((value & 0x00FF00) >> 8) as u8,
                    (value & 0x0000FF) as u8,
                ]
            },
            original: None,
            original_sha256: value.original_sha256,
            properties: value.properties.into(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
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

impl From<ImageMetaScheme> for ImageMeta {
    fn from(value: ImageMetaScheme) -> Self {
        Self {
            width: value.width,
            height: value.height,
            extension: value.ext,
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

impl From<PropertiesSchema> for Properties {
    fn from(value: PropertiesSchema) -> Self {
        Self {
            machine: value.machine,
            lens: value.lens,
            gps_lat_lng: value.gps_lat_lng.and_then(|lnglat| {
                lnglat
                    .first()
                    .and_then(|lng| lnglat.get(1).map(|lat| (*lng, *lat)))
            }),
            f_number: value.f_number,
            shutter_speed: value.shutter_speed.map(NormalizedRational),
            shutter_speed_controlled: value.shutter_speed_controlled,
            iso: value.iso,
            focal: value.focal,
            orientation: None,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PhotoReferenceSchema {
    id: Identifier,
    federator: Option<EntityName>,
    year: i32,
    month: u32,
    original_sha256: String,
    images: HashMap<String, ImageMetaScheme>,
    tags: HashMap<String, Vec<String>>,
    shot_time: String,
    representative_color: String,
}

impl From<PhotoReference> for PhotoReferenceSchema {
    fn from(value: PhotoReference) -> Self {
        PhotoReferenceSchema {
            year: value.id().year,
            month: value.id().month,
            id: value.id().clone(),
            federator: value.origin.federator().cloned(),
            original_sha256: value.hash,
            shot_time: value.shot_time.to_rfc3339(),
            images: value
                .images
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            tags: value.tags,
            representative_color: {
                let [r, g, b] = value.representative_rgb;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            },
        }
    }
}

impl From<PhotoReferenceSchema> for PhotoReference {
    fn from(value: PhotoReferenceSchema) -> Self {
        let origin = if let Some(federator) = value.federator {
            PhotoOrigin::Federated(RemoteOrigin {
                federator,
                identifier: value.id,
            })
        } else {
            PhotoOrigin::Local(LocalIdentifier(value.id))
        };

        PhotoReference {
            origin,
            year: value.year,
            month: value.month,
            hash: value.original_sha256,
            shot_time: chrono::DateTime::parse_from_rfc3339(&value.shot_time).unwrap(),
            images: value
                .images
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            tags: value.tags,
            representative_rgb: {
                let value: u64 = u64::from_str_radix(&value.representative_color[1..], 16).unwrap();
                [
                    ((value & 0xFF0000) >> 16) as u8,
                    ((value & 0x00FF00) >> 8) as u8,
                    (value & 0x0000FF) as u8,
                ]
            },
        }
    }
}
