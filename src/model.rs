use std::{collections::HashMap, fmt, str::FromStr};

use chrono::{DateTime, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Identifier {
    pub year: i32,
    pub month: u32,
    ulid: String,
}

impl Identifier {
    pub fn new(shot_date: &DateTime<FixedOffset>, ulid: &str) -> Self {
        Identifier {
            year: shot_date.year(),
            month: shot_date.month(),
            ulid: ulid.to_string(),
        }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}{:02}-{}", self.year, self.month, self.ulid)
    }
}

impl Serialize for Identifier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for Identifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format: {year:04}{month:02}-{ulid}
        if s.len() < 8 {
            anyhow::bail!("Identifier string too short");
        }

        let year: i32 = s[..4].parse()?;
        let month: u32 = s[4..6].parse()?;
        let ulid = s[7..].to_string();

        Ok(Identifier { year, month, ulid })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoMeta {
    pub id: Identifier,
    pub original: ImageMeta,
    pub images: HashMap<String, ImageMeta>,
    pub original_sha256: String,
    pub properties: Properties,
    pub shot_time: DateTime<FixedOffset>,
    pub representative_rgb: [u8; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginalImageMeta {
    pub width: u32,
    pub height: u32,
    pub extension: String,
    pub mime: String,
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
