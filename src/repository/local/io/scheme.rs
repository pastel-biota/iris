#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
pub enum VersionedPhotoMetaScheme {
    V1(v1::PhotoMetaScheme),
}

pub mod v1 {
    use std::collections::HashMap;
    
    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Serialize};
    
    use crate::symmetrical_from_into;
    use crate::model::{self, ImageMeta, NormalizedRational, Orientation, PhotoOrigin};
    
    symmetrical_from_into! {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct PhotoMetaScheme (= model::PhotoMeta) {
            pub origin: PhotoOrigin,
            pub original: Option<ImageMeta>,
            pub images: HashMap<String, ImageMetaScheme> |x| ->
                HashMap<String, ImageMetaScheme>,
                HashMap<String, model::ImageMeta>
            => x.into_iter().map(|(k, v)| (k, v.into())).collect::<Rty>(),
            #[serde(default)]
            pub tags: HashMap<String, Vec<String>>,
            pub original_sha256: String,
            pub properties: PropertiesScheme,
            pub shot_time: DateTime<FixedOffset>,
            pub representative_rgb: [u8; 3],
        }
    }
    
    symmetrical_from_into! {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct ImageMetaScheme (= model::ImageMeta) {
            pub width: u32,
            pub height: u32,
            pub extension: String,
            pub mime: String,
        }
    }

    symmetrical_from_into! {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct PropertiesScheme (= model::Properties) {
            pub gps_lat_lng: Option<(f64, f64)>,
            pub machine: String,
            pub lens: Option<String>,
            pub f_number: Option<f64>,
            pub shutter_speed: Option<NormalizedRational>,
            pub shutter_speed_controlled: Option<bool>,
            pub iso: Option<u64>,
            pub focal: Option<f64>,
            pub orientation: Option<Orientation>,
        }
    }
}
