use serde::{Deserialize, Serialize};

use crate::{repository::io::ScopedPath, tag::types::KnownTags};

pub struct KnownTagsRegistry {
    scope: ScopedPath,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
pub enum TagRegistryPayload {
    V1(v1::TagRegistryPayload),
}

mod v1 {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    use crate::{
        model::{Tag, TagGroup, TagIdentifier, TagName},
        tag::json::KnownTags,
    };

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct TagRegistryPayload {
        groups: HashMap<TagName, TagGroupDefinition>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct TagGroupDefinition {
        display: String,
        tags: HashMap<TagName, TagDefinition>,
        annotations: Option<HashMap<String, serde_json::Value>>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct TagDefinition {
        display: String,
        annotations: Option<HashMap<String, serde_json::Value>>,
    }

    impl From<TagRegistryPayload> for KnownTags {
        fn from(value: TagRegistryPayload) -> Self {
            let tag_groups = value
                .groups
                .clone()
                .into_iter()
                .map(|(name, group)| TagGroup {
                    name: name,
                    display: group.display,
                    annotaions: group.annotations.unwrap_or_else(|| HashMap::new()),
                })
                .collect::<Vec<_>>();

            let tags = value
                .groups
                .into_iter()
                .flat_map(|(group_name, group)| {
                    group.tags.into_iter().map(move |(tag_name, tag)| Tag {
                        ident: TagIdentifier(group_name.clone(), tag_name),
                        display: tag.display,
                        annotaions: tag.annotations.unwrap_or_else(|| HashMap::new()),
                    })
                })
                .collect::<Vec<_>>();

            KnownTags { tags, tag_groups }
        }
    }
}

impl KnownTagsRegistry {
    pub fn new(scope: ScopedPath) -> Self {
        KnownTagsRegistry { scope }
    }

    pub async fn get(&self) -> anyhow::Result<KnownTags> {
        let path = self.scope.join("_def/tags.json");

        let TagRegistryPayload::V1(payload) =
            serde_json::from_slice(path.read_binary()?.as_slice())?;

        Ok(payload.into())
    }
}
