use std::{fmt, str::FromStr};

use chrono::{DateTime, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Identifier {
    pub year: i32,
    pub month: u32,
    file_name: String,
    ulid: String,
}

impl Identifier {
    pub fn new(shot_date: &DateTime<FixedOffset>, file_name: &str, ulid: &str) -> Self {
        Identifier {
            year: shot_date.year(),
            month: shot_date.month(),
            file_name: file_name.to_lowercase().replace('.', "_"),
            ulid: ulid.to_string(),
        }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}{:02}_{}-{}",
            self.year, self.month, self.file_name, self.ulid
        )
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
        // Format: {year:04}{month:02}_{file_name}-{ulid}
        if s.len() < 8 {
            anyhow::bail!("Identifier string too short");
        }

        let year: i32 = s[..4].parse()?;
        let month: u32 = s[4..6].parse()?;

        if s.as_bytes()[6] != b'_' {
            anyhow::bail!("Expected '_' at position 6");
        }

        let rest = &s[7..];
        let last_dash = rest
            .rfind('-')
            .ok_or_else(|| anyhow::anyhow!("Expected '-' separator between file_name and ulid"))?;

        let file_name = rest[..last_dash].to_string();
        let ulid = rest[last_dash + 1..].to_string();

        Ok(Identifier {
            year,
            month,
            file_name,
            ulid,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhotoMeta {
    pub id: Identifier,
    pub images: Vec<ImageMeta>,
    pub original_sha256: String,
    pub properties: Properties,
    pub shot_time: DateTime<FixedOffset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageMeta {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub image_id: String,
    pub extension: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Properties {
    pub machine: String,
    pub lens: String,
    pub gps_lng_lat: Option<(f32, f32)>,
}
