use std::{borrow::Cow, fmt::Display, ops::Deref, str::FromStr};

use serde_with::{DeserializeFromStr, SerializeDisplay};
use utoipa::openapi::{RefOr, Schema};

#[derive(Clone, Debug, Hash, Eq, PartialEq, DeserializeFromStr, SerializeDisplay)]
pub struct TagName(pub String);

impl Deref for TagName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for TagName {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.is_empty() {
            anyhow::bail!("The tag group / tag name cannot be empty");
        }

        if !raw
            .chars()
            .all(|char: char| char == '_' || char.is_ascii_alphanumeric())
        {
            anyhow::bail!(
                "The tag group / tag name can only contain alphanumeric character or underscore"
            );
        }

        Ok(TagName(raw.to_string()))
    }
}

impl Display for TagName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, DeserializeFromStr, SerializeDisplay)]
pub struct TagIdentifier(pub TagName, pub TagName);

impl FromStr for TagIdentifier {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.is_empty() {
            anyhow::bail!("The tag identifier cannot be empty");
        }

        let Some((group, name)) = raw.split_once("/") else {
            anyhow::bail!("The tag identifier must be in the form of 'group/tag'");
        };

        Ok(TagIdentifier(group.parse()?, name.parse()?))
    }
}

impl Display for TagIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

impl utoipa::PartialSchema for TagIdentifier {
    fn schema() -> RefOr<Schema> {
        utoipa::openapi::Object::builder()
            .schema_type(utoipa::openapi::Type::String)
            .examples(["rating/4", "rating/5", "location/park"])
            .into()
    }
}

impl utoipa::ToSchema for TagIdentifier {
    fn name() -> Cow<'static, str> {
        "TagIdentifier".into()
    }
}

#[derive(Clone, Debug)]
pub struct TagGroup {
    pub name: TagName,
    pub display: String,
}

impl Display for TagGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug)]
pub struct Tag {
    pub ident: TagIdentifier,
    pub display: String,
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)
    }
}
