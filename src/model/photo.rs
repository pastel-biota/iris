use std::{collections::HashMap, fmt::Display};

use chrono::{DateTime, FixedOffset};
use json_patch::jsonptr::Pointer;
use serde::{Deserialize, Serialize};

use crate::model::{Identifier, PhotoOrigin, TagIdentifier};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoMeta {
    pub origin: PhotoOrigin,
    pub original: Option<ImageMeta>,
    pub images: HashMap<String, ImageMeta>,
    pub tags: Vec<TagIdentifier>,
    pub original_sha256: String,
    pub properties: Properties,
    pub shot_time: DateTime<FixedOffset>,
    pub representative_rgb: [u8; 3],
}

#[derive(Debug, Hash, Eq)]
pub struct ControlledField {
    pub pointer: &'static Pointer,
    pub reason: &'static str,
}

impl ControlledField {
    pub const fn new(pointer: &'static str, reason: &'static str) -> Self {
        Self {
            pointer: Pointer::from_static(pointer),
            reason,
        }
    }
}

impl Display for ControlledField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.pointer, self.reason)
    }
}

impl PartialEq for ControlledField {
    fn eq(&self, other: &Self) -> bool {
        self.pointer == other.pointer
    }
}

pub const CONTROLLED_FIELDS: &[ControlledField] = &[
    ControlledField::new(
        "/origin",
        "This field contains ID and the photo's ownership, which is immutable",
    ),
    ControlledField::new(
        "/original",
        "This field contains the uploaded photo's image metadata, which is immutable",
    ),
    ControlledField::new(
        "/images",
        "This field contains generated image's metadata, which is immutable",
    ),
    ControlledField::new(
        "/original_sha256",
        "This field contains SHA256 hash of the uploaded photo's image, which is immutable",
    ),
];

impl PhotoMeta {
    pub fn id(&self) -> &Identifier {
        self.origin.id()
    }

    pub fn controlled_field(pointer: &Pointer) -> Option<&'static ControlledField> {
        CONTROLLED_FIELDS
            .iter()
            .find(|ptr| pointer.starts_with(ptr.pointer))
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
