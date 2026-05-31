use std::{collections::HashMap, fmt::{self, Display}, ops::Deref, str::FromStr};

use anyhow::bail;
use chrono::{DateTime, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, utoipa::ToSchema)]
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

        if ulid.contains(|char: char| !char.is_ascii_alphanumeric()) {
            anyhow::bail!("ULID part contains invalid character");
        }

        Ok(Identifier { year, month, ulid })
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct LocalIdentifier(pub Identifier);

impl Display for LocalIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for LocalIdentifier {
    type Target = Identifier;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoMeta {
    pub origin: PhotoOrigin,
    pub original: Option<ImageMeta>,
    pub images: HashMap<String, ImageMeta>,
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PhotoReference {
    pub origin: PhotoOrigin,
    pub year: i32,
    pub month: u32,
    pub hash: String,
    pub images: HashMap<String, ImageMeta>,
    pub shot_time: DateTime<FixedOffset>,
    pub representative_rgb: [u8; 3],
}

impl PhotoReference {
    pub fn id(&self) -> &Identifier {
        self.origin.id()
    }
}

impl From<PhotoMeta> for PhotoReference {
    fn from(value: PhotoMeta) -> Self {
        Self {
            year: value.origin.id().year,
            month: value.origin.id().month,
            origin: value.origin,
            hash: value.original_sha256,
            images: value
                .images
                .into_iter()
                .collect(),
            shot_time: value.shot_time,
            representative_rgb: value.representative_rgb,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum PhotoOrigin {
    Local(LocalIdentifier),
    Federated(RemoteOrigin)
} 

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RemoteOrigin {
    pub federator: EntityName,
    pub identifier: Identifier,
}

impl PhotoOrigin {
    pub fn local(&self) -> bool {
        matches!(self, PhotoOrigin::Local(_))
    }

    pub fn federated(&self) -> bool {
        matches!(self, PhotoOrigin::Federated { .. })
    }

    pub fn local_id(&self) -> Option<&LocalIdentifier> {
        match self {
            Self::Local(id) => Some(id),
            Self::Federated { .. } => None,
        }
    }

    pub fn id(&self) -> &Identifier {
        match self {
            Self::Local(id) => &id.0,
            Self::Federated(origin) => &origin.identifier,
        }
    }

    pub fn federator(&self) -> Option<&EntityName> {
        match self {
            Self::Local(_) => None,
            Self::Federated(origin) => Some(&origin.federator),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct EntityName(String);

impl Display for EntityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for EntityName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for EntityName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.chars().all(|ch: char| ch.is_ascii_alphabetic() || ch == '-' || ch == '_') {
            bail!("There is a invalid character - only 0..9, a..z, A..Z, hyphen, underscore can be used")
        }

        Ok(EntityName(s.to_string()))
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Whitelist {
    Select { pics: Vec<Identifier>, },
    Everything,
}

impl Whitelist {
    pub fn new_selective() -> Self {
        Self::Select { pics: Vec::new() }
    }

    pub fn allow_photos(self, photos: &[Identifier]) -> Self {
        match self {
            Whitelist::Select { mut pics } => {
                pics.extend(photos.iter().cloned());
                Whitelist::Select { pics }
            },
            Whitelist::Everything => self,
        }
    }

    pub fn is_allowed(&self, id: &Identifier) -> bool {
        match self {
            Self::Select { pics } => pics.contains(id),
            Self::Everything => true,
        }
    }

    pub fn seleted_pics(&self) -> Option<&[Identifier]> {
        match self {
            Self::Select { pics } => Some(pics),
            Self::Everything => None,
        }
    }
}

