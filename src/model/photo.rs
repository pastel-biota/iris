use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::model::{Identifier, PhotoOrigin};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoMeta {
    pub origin: PhotoOrigin,
    pub original: Option<ImageMeta>,
    pub images: HashMap<String, ImageMeta>,
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
    pub original_sha256: String,
    pub properties: Properties,
    pub shot_time: DateTime<FixedOffset>,
    pub representative_rgb: [u8; 3],
}

impl PhotoMeta {
    pub fn id(&self) -> &Identifier {
        self.origin.id()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub extension: String,
    pub mime: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Properties {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedRational(pub f32);

#[derive(Debug)]
pub struct Rational(pub i32, pub i32);

impl Rational {
    pub fn to_f64(&self) -> f64 {
        let Rational(denom, num) = self;
        (*num as f64) / (*denom as f64)
    }

    pub fn normalize_to_one(&self) -> NormalizedRational {
        let Rational(denom, num) = self;

        let new_num = (*denom as f32) / (*num as f32);
        NormalizedRational(new_num)
    }
}

#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub struct Orientation {
    pub rotation: Rotation,
    pub flip: bool,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rotation {
    #[default]
    Upright,
    UpsideDown,
    CounterClockwise,
    Clockwise,
}
