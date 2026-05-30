use std::collections::HashMap;

use anyhow::{Context as _, bail};

use crate::{
    model::{Identifier, ImageMeta, PhotoMeta, PhotoReference}, repository::{io::ScopedPath, photo_index::PhotoIndexProvider},
};

#[derive(Debug)]
pub struct OriginalSha256Index {
    path: ScopedPath,
    content: Option<IndexEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "_v", rename_all = "lowercase")]
pub enum IndexEntry {
    V1(IndexEntryV1),
}

impl Default for IndexEntry {
    fn default() -> Self {
        IndexEntry::V1(Default::default())
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct IndexEntryV1 {
    total_count: u32,
    pics: HashMap<String, PhotoReference>,
}

impl PhotoIndexProvider for OriginalSha256Index {
    const INDEX_NAME: &'static str = "sha256 index";
    type Entry = IndexEntry;

    fn add_photo(&mut self, photo: &PhotoReference) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        index
            .pics
            .insert(photo.hash.clone(), photo.clone().into());
        index.total_count += 1;

        self.save()?;

        Ok(())
    }

    fn add_new_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let photo = index
            .pics
            .values_mut()
            .find(|photo| photo.id() == photo_id)
            .context("The image was not found")?;

        photo
            .images
            .insert(image_id.to_string(), image.clone());

        self.save()?;

        Ok(())
    }

    fn total_count(&mut self) -> anyhow::Result<u32> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.total_count)
    }
}

impl OriginalSha256Index {
    pub fn new(path: &ScopedPath) -> OriginalSha256Index {
        OriginalSha256Index {
            path: path.clone(),
            content: None,
        }
    }

    pub fn get_photo_ref(&mut self, id: &Identifier) -> anyhow::Result<Option<&PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(index.pics
            .iter()
            .find(|(_id, photo_ref)| photo_ref.id() == id)
            .map(|(_id, photo_ref)| photo_ref))
    }

    pub fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.pics.contains_key(hash))
    }

    pub fn list_photos_from_ids_list(
        &mut self,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<&PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        // FIXME: This is O(n^2)! 
        Ok(ids
            .iter()
            .flat_map(|id| index.pics.values().find(|photo| photo.id() == id))
            .collect::<Vec<_>>())
    }

    pub fn get_photos_list_from_hashes_list<'s, 'h>(
        &'s mut self,
        hashes: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        let IndexEntry::V1(index) = self.load_mut()?;

        Ok(hashes
            .iter()
            .flat_map(|hash| index.pics.get(hash).map(|photo| (hash.as_str(), photo)))
            .collect::<HashMap<_, _>>())
    }

    pub fn delete_photo(&mut self, id: &Identifier, hash: &str) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        let photo = index.pics
            .get(hash)
            .expect("TBI - The photo was found in the all index but the photo with the found has is not found in the hash index");

        if photo.id() != id {
            bail!("The photo with the found hash was found but it is not the right photo: (requested) {} !=  (found) {}", id, photo.id())
        }

        index.pics.remove(hash).expect("Just checked above");

        Ok(())
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
            serde_json::to_vec_pretty(entry)
                .context("Failed to serialize the sha256 index")?
        };

        self.path
            .write(bytes)
            .context("Failed to write the sha256 index")?;

        Ok(())
    }
}
