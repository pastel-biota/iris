pub mod date;
pub mod original_hash;
pub mod reference;

use std::collections::HashMap;

use anyhow::Context as _;

use crate::{
    model::{Identifier, ImageMeta, LocalIdentifier, PhotoReference},
    repository::{io::ScopedPath, photo_index::reference::ReferenceIndex},
};

use self::{date::DateImageIndex, original_hash::OriginalSha256Index};

pub struct PhotoIndex {
    all_index: DateImageIndex,
    hash_index: OriginalSha256Index,
    reference_index: ReferenceIndex,
}

impl PhotoIndex {
    pub fn new(base_dir: &ScopedPath) -> Self {
        PhotoIndex {
            all_index: DateImageIndex::new(&base_dir.join("date.json")),
            hash_index: OriginalSha256Index::new(&base_dir.join("sha256.json")),
            reference_index: ReferenceIndex::new(&base_dir.join("_pics.json")),
        }
    }

    pub fn add_new_photo(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        self.all_index.upsert(photo)?;
        self.hash_index.upsert(photo)?;
        self.reference_index.upsert(photo)?;

        assert!(self.all_index.total_count()? == self.hash_index.total_count()?);

        Ok(())
    }

    pub fn add_new_image(
        &mut self,
        photo_id: &LocalIdentifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()> {
        self.reference_index
            .add_new_image(photo_id, image_id, image)?;

        Ok(())
    }

    pub fn upsert_photo(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        self.all_index.upsert(photo)?;
        self.hash_index.upsert(photo)?;
        self.reference_index.upsert(photo)?;

        assert!(self.all_index.total_count()? == self.hash_index.total_count()?);

        Ok(())
    }

    pub fn get_photo_ref(
        &mut self,
        photo_id: &Identifier,
    ) -> anyhow::Result<Option<PhotoReference>> {
        self.reference_index.get_photo(photo_id)
    }

    pub fn list_images(
        &mut self,
        beginning: Option<&Identifier>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        let photo_ids = if let Some(beginning) = beginning {
            self.all_index
                .list_images_beginning_from_photo(beginning, size)?
        } else {
            self.all_index.list_first_n_images(size)?
        };

        self.reference_index.bulk_load_photo(photo_ids.as_slice().iter().map(|x| x.id()))?
            .map(|maybe_photo| maybe_photo.ok_or(anyhow::anyhow!("Some photos are not registered although found in the index, the state might be broken")))
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool> {
        self.hash_index.image_exists_with_hash(hash)
    }

    pub fn get_photos_list_from_hashes_list<'s, 'h>(
        &'s mut self,
        hashes: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        let photo_ids = self.hash_index.get_photos_list_from_hashes_list(hashes)?;

        let photo_refs = self
            .reference_index
            .bulk_load_photo_map(photo_ids.values().map(|x| x.id()))?;

        let mut photo_ref_by_hash = HashMap::new();
        for hash in hashes {
            let id = photo_ids.get(hash.as_str());
            let photo = id.and_then(|&orgiin| photo_refs.get(orgiin.id()));

            photo_ref_by_hash.insert(hash, photo);
        }

        todo!();
    }

    pub fn get_photos_list_by_ids_list(
        &mut self,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<PhotoReference>> {
        let photo_ids = self.hash_index.list_photos_from_ids_list(ids)?;

        self.reference_index.bulk_load_photo(photo_ids.as_slice().iter().map(|x| x.id()))?
            .map(|maybe_photo| maybe_photo.ok_or(anyhow::anyhow!("Some photos are not registered although found in the index, the state might be broken")))
            .collect::<Result<Vec<_>, _>>()
    }

    pub fn delete_photo(&mut self, photo_id: &Identifier) -> anyhow::Result<()> {
        let photo = self.reference_index.delete_photo(photo_id)?;

        self.all_index.delete_photo(photo_id)?;
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
