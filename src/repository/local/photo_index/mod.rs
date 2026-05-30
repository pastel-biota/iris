pub mod all;
pub mod original_hash;

use std::collections::HashMap;

use anyhow::Context as _;

use crate::{model::{Identifier, ImageMeta, PhotoMeta, PhotoReference}, repository::io::ScopedPath};

use self::{all::AllImageIndex, original_hash::OriginalSha256Index};

pub struct PhotoIndex {
    all_index: AllImageIndex,
    hash_index: OriginalSha256Index,
}

impl PhotoIndex {
    pub fn new(base_dir: &ScopedPath) -> Self {
        PhotoIndex {
            all_index: AllImageIndex::new(&base_dir.join("all.json")),
            hash_index: OriginalSha256Index::new(&base_dir.join("sha256.json")),
        }
    }

    pub fn add_new_photo(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        self.all_index.upsert(photo)?;
        self.hash_index.upsert(photo)?;

        assert!(self.all_index.total_count()? == self.hash_index.total_count()?);

        Ok(())
    }

    pub fn add_new_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()> {
        self.all_index.add_new_image(photo_id, image_id, image)?;
        self.hash_index.add_new_image(photo_id, image_id, image)?;

        Ok(())
    }

    pub fn upsert_photo(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        self.all_index.upsert(photo)?;
        self.hash_index.upsert(photo)?;

        assert!(self.all_index.total_count()? == self.hash_index.total_count()?);

        Ok(())
    }

    pub fn get_photo_ref(&mut self, photo_id: &Identifier) -> anyhow::Result<Option<&PhotoReference>> {
        self.hash_index.get_photo_ref(photo_id)
    }

    pub fn list_images(
        &mut self,
        beginning: Option<&Identifier>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        if let Some(beginning) = beginning {
            self.all_index.list_images_beginning_from_photo(beginning, size)
        } else {
            self.all_index.list_first_n_images(size)
        }
    }

    pub fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool> {
        self.hash_index.image_exists_with_hash(hash)
    }

    pub fn get_photos_list_from_hashes_list<'s, 'h>(
        &'s mut self,
        hash: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        self.hash_index
            .get_photos_list_from_hashes_list(hash)
    }

    pub fn get_photos_list_by_ids_list<'s>(
        &'s mut self,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<&'s PhotoReference>> {
        self.hash_index
            .list_photos_from_ids_list(ids)
    }

    pub fn delete_photo(&mut self, photo_id: &Identifier) -> anyhow::Result<()> {
        let photo = self.all_index.delete_photo(photo_id)?;
        self.hash_index.delete_photo(photo_id, &photo.hash)?;

        Ok(())
    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        self.all_index.total_count()
    }
}

pub trait PhotoIndexProvider {
    const INDEX_NAME: &'static str;
    type Entry: serde::Serialize + serde::de::DeserializeOwned + Default + std::fmt::Debug;

    fn upsert(&mut self, photo: &PhotoReference) -> anyhow::Result<()>;
    fn add_new_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()>;
    fn total_count(&mut self) -> anyhow::Result<u32>;

    fn load_to_file(&mut self, path: &ScopedPath) -> anyhow::Result<Self::Entry> {
        if !path.exists() {
            return self.init(path);
        }

        let bytes = path.read_binary().context(format!(
            "Failed to open a file for the {}",
            Self::INDEX_NAME
        ))?;

        serde_json::from_slice(&bytes)
            .context(format!("The {} contains invalid content", Self::INDEX_NAME))
    }

    fn init(&mut self, path: &ScopedPath) -> anyhow::Result<Self::Entry> {
        let value = Self::Entry::default();
        let bytes = serde_json::to_vec_pretty(&value).context(format!(
            "Failed to serialize an empty entry for the {}",
            Self::INDEX_NAME
        ))?;

        path.write(bytes).context(format!(
            "Failed to create a file for the {}",
            Self::INDEX_NAME
        ))?;

        Ok(value)
    }
}
