use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Context as _;

use crate::ingest::{
    infra::photo_index::{PhotoIndexProvider, PhotoReference},
    model::{Identifier, ImageMeta, PhotoMeta},
};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct OriginalSha256Index {
    path: PathBuf,
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

    fn add_photo(&mut self, photo: &PhotoMeta) -> anyhow::Result<()> {
        let IndexEntry::V1(index) = self.load_mut()?;

        index
            .pics
            .insert(photo.original_sha256.clone(), photo.clone().into());
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
            .find(|photo| &photo.id == photo_id)
            .context("The image was not found")?;
        photo
            .images
            .insert(image_id.to_string(), image.clone().into());

        self.save()?;

        Ok(())
    }

    fn total_count(&mut self) -> anyhow::Result<u32> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.total_count)
    }
}

impl OriginalSha256Index {
    pub fn new(path: &Path) -> OriginalSha256Index {
        OriginalSha256Index {
            path: path.to_path_buf(),
            content: None,
        }
    }

    pub fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool> {
        let IndexEntry::V1(index) = self.load_mut()?;
        Ok(index.pics.contains_key(hash))
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

    fn load_mut(&mut self) -> anyhow::Result<&mut IndexEntry> {
        if self.content.is_none() {
            let path = self.path.clone();
            let entry = self.load_to_file(&path)?;
            return Ok(self.content.insert(entry));
        }

        Ok(self.content.as_mut().unwrap())
    }

    fn save(&mut self) -> anyhow::Result<()> {
        let mut file =
            File::create(&self.path).context("Failed to create a file for the sha256 index")?;

        serde_json::to_writer_pretty(&mut file, &self.load_mut()?)
            .context("Failed to write a empty entry for the sha256 indexx")?;

        Ok(())
    }
}
