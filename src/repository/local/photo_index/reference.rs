use std::collections::HashMap;

use anyhow::Context as _;

use crate::{
    model::{Identifier, ImageMeta, LocalIdentifier, PhotoReference},
    repository::{io::ScopedPath, photo_index::PhotoIndexProvider},
};

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

    fn upsert(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let replaced = index.pics.insert(photo.id().clone(), photo.clone().into());

        if replaced.is_none() {
            index.total_count += 1;
        }

        self.save()?;

        Ok(())
    }

    fn total_count(&mut self) -> anyhow::Result<u32> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.total_count)
    }
}

impl ReferenceIndex {
    pub fn new(path: &ScopedPath) -> ReferenceIndex {
        ReferenceIndex {
            path: path.clone(),
            content: None,
        }
    }

    pub fn get_photo(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(
            index.pics.get(id).cloned()
                .map(|v1::VersionedPhotoReference::V1(refs)| refs.into())
        )
    }

    pub fn bulk_load_photo_map<'a>(
        &mut self,
        id: impl IntoIterator<Item = &'a Identifier>,
    ) -> anyhow::Result<HashMap<Identifier, PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(id
            .into_iter()
            .filter_map(|id| {
                index
                    .pics
                    .get(&*id)
                    .map(|v1::VersionedPhotoReference::V1(photo)| (
                            (*id).clone(), photo.clone().into()
                    ))
            })
            .collect())
    }

    pub fn bulk_load_photo<'a>(
        &mut self,
        id: impl IntoIterator<Item = &'a Identifier>,
    ) -> anyhow::Result<impl Iterator<Item = Option<PhotoReference>>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(id
            .into_iter()
            .map(|id| index.pics.get(&*id).map(|v1::VersionedPhotoReference::V1(photo)| photo.clone().into())))
    }

    pub fn add_new_image(
        &mut self,
        photo_id: &LocalIdentifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let v1::VersionedPhotoReference::V1(photo) = index
            .pics
            .get_mut(&photo_id.0)
            .context("The image was not found")?;

        photo
            .images
            .insert(image_id.to_string(), image.clone().into());

        self.save()?;

        Ok(())
    }

    pub fn delete_photo(&mut self, photo_id: &Identifier) -> anyhow::Result<PhotoReference> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let v1::VersionedPhotoReference::V1(photo) = index
            .pics
            .remove(&photo_id)
            .context("The image was not found")?;

        Ok(photo.into())
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
