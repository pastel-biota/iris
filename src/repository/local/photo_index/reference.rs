use std::collections::HashMap;

use anyhow::Context as _;

use crate::{
    model::{self, Identifier, ImageMeta, LocalIdentifier, PhotoReference}, repository::{io::ScopedPath},
};
use crate::repository::photo_index::PhotoIndexProvider;

pub struct ReferenceIndex {
    path: ScopedPath,
    content: Option<IndexEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
pub enum IndexEntry {
    V1(v1::IndexEntry),
}

impl Default for IndexEntry {
    fn default() -> Self {
        IndexEntry::V1(Default::default())
    }
}

mod v1 {
    use crate::{model::{self, Identifier}, symmetrical_from_into};
    use chrono::{DateTime, FixedOffset};
    use std::collections::HashMap;

    #[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
    pub(super) struct IndexEntry {
        pub total_count: u32,
        pub pics: HashMap<Identifier, VersionedPhotoReference>,
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    #[serde(tag = "_v", rename_all = "lowercase")]
    pub enum VersionedPhotoReference {
        V1(PhotoReference),
    }

    impl From<model::PhotoReference> for VersionedPhotoReference {
        fn from(value: model::PhotoReference) -> Self {
            VersionedPhotoReference::V1(value.into())
        }
    }

    symmetrical_from_into! {
        #[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
        pub struct PhotoReference (= model::PhotoReference) {
            pub origin: model::PhotoOrigin,
            pub year: i32,
            pub month: u32,
            pub hash: String,
            pub images: HashMap<String, ImageMeta> |x| ->
                HashMap<String, ImageMeta>,
                HashMap<String, model::ImageMeta>
            => x.into_iter().map(|(k, v)| (k, v.into())).collect::<Rty>(),
            #[serde(default)]
            pub tags: HashMap<String, Vec<String>>,
            pub shot_time: DateTime<FixedOffset>,
            pub representative_rgb: [u8; 3],
        }
    }

    symmetrical_from_into! {
        #[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
        pub struct ImageMeta (= model::ImageMeta) {
            pub width: u32,
            pub height: u32,
            pub extension: String,
            pub mime: String,
        }
    }
}

impl PhotoIndexProvider for ReferenceIndex {
    const INDEX_NAME: &'static str = "sha256 index";
    type Entry = IndexEntry;
}

impl ReferenceIndex {
    pub fn new(path: &ScopedPath) -> ReferenceIndex {
        ReferenceIndex {
            path: path.clone(),
            content: None,
        }
    }

    pub fn dangerously_read_photo(&mut self) -> anyhow::Result<impl Iterator<Item = model::PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(index
            .pics
            .values()
            .map(|v1::VersionedPhotoReference::V1(photo)| photo.clone().into()))
    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.total_count)
    }

    fn load_mut(&mut self) -> anyhow::Result<&mut IndexEntry> {
        if self.content.is_none() {
            let path = self.path.clone();
            let entry = self.load_to_file(&path)?;
            return Ok(self.content.insert(entry));
        }

        Ok(self.content.as_mut().unwrap())
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let bytes = {
            let entry = self.load_mut()?;
            serde_json::to_vec_pretty(entry).context("Failed to serialize the sha256 index")?
        };

        self.path
            .write(bytes)
            .context("Failed to write the sha256 index")?;

        Ok(())
    }
}
