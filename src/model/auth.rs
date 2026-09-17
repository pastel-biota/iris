use std::{
    fmt::{self, Display},
    ops::Deref,
    str::FromStr,
};

use anyhow::bail;

use crate::model::Identifier;

#[derive(
    Clone, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
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
        if !s
            .chars()
            .all(|ch: char| ch.is_ascii_alphabetic() || ch == '-' || ch == '_')
        {
            bail!(
                "There is a invalid character - only 0..9, a..z, A..Z, hyphen, underscore can be used"
            )
        }

        Ok(EntityName(s.to_string()))
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Whitelist {
    Select { pics: Vec<Identifier> },
}

impl Whitelist {
    pub fn new_selective() -> Self {
        Self::Select { pics: Vec::new() }
    }
}

/// The set of photos a repository query may return.
///
/// This is intentionally separate from whether request authorization was bypassed: both a
/// bypassed request and an authenticated entity with full image access can see all photos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhotoVisibility {
    All,
    WhitelistedFor(EntityName),
}
