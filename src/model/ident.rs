use std::{
    borrow::Cow,
    fmt::{self, Display},
    ops::Deref,
    str::FromStr,
};

use chrono::{DateTime, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};
use utoipa::openapi::{RefOr, Schema};

use crate::model::EntityName;

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

        if ulid.contains(|char: char| !char.is_ascii_alphanumeric()) {
            anyhow::bail!("ULID part contains invalid character");
        }

        Ok(Identifier { year, month, ulid })
    }
}

impl utoipa::PartialSchema for Identifier {
    fn schema() -> RefOr<Schema> {
        utoipa::openapi::Object::builder()
            .schema_type(utoipa::openapi::Type::String)
            .examples(["202604-01ARZ3NDEKTSV4RRFFQ69G5FAV"])
            .into()
    }
}

impl utoipa::ToSchema for Identifier {
    fn name() -> Cow<'static, str> {
        "Identifier".into()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum PhotoOrigin {
    Local(LocalIdentifier),
    Federated(RemoteOrigin),
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
