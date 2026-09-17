use std::collections::HashMap;

use crate::model::{Tag, TagGroup};

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TagRegistryResponse {
    pub tags: Vec<TagDescriptionScheme>,
    pub tag_groups: Vec<TagGroupDescriptionScheme>,
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TagDescriptionScheme {
    pub ident: String,
    pub segments: (String, String),
    pub group_display: Option<String>,
    pub display: String,
    pub annotations: HashMap<String, serde_json::Value>,
}

impl TagDescriptionScheme {
    pub fn new(tag_group: Option<&TagGroup>, tag: Tag) -> anyhow::Result<Self> {
        Ok(Self {
            ident: tag.ident.to_string(),
            segments: (tag.ident.0.to_string(), tag.ident.1.to_string()),
            display: tag.display,
            group_display: tag_group
                .map(|group| {
                    if group.name == tag.ident.0 {
                        Ok(group.display.clone())
                    } else {
                        anyhow::bail!("programming bug: The group does not match")
                    }
                })
                .transpose()?,
            annotations: tag.annotaions,
        })
    }
}

#[derive(Clone, Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TagGroupDescriptionScheme {
    pub name: String,
    pub display: String,
    pub annotations: HashMap<String, serde_json::Value>,
}

impl From<TagGroup> for TagGroupDescriptionScheme {
    fn from(value: TagGroup) -> Self {
        Self {
            name: value.name.to_string(),
            display: value.display,
            annotations: value.annotaions,
        }
    }
}
