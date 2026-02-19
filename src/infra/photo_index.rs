pub mod all;
pub mod original_hash;

use std::{collections::HashMap, fs::File, path::Path};

use anyhow::Context as _;
use chrono::{DateTime, FixedOffset};

use crate::{infra::photo_index::{all::AllImageIndex, original_hash::OriginalSha256Index}, model::{Identifier, ImageMeta, PhotoMeta}};

pub struct PhotoIndex {
    all_index: AllImageIndex,
    hash_index: OriginalSha256Index,
}

impl PhotoIndex {
    pub fn new(base_dir: &Path) -> Self {
        PhotoIndex {
            all_index: AllImageIndex::new(&base_dir.join("all.json")),
            hash_index: OriginalSha256Index::new(&base_dir.join("sha256.json")),
        }
    }

    pub fn add_new_image(&mut self, photo: &PhotoMeta) -> anyhow::Result<()> {
        self.all_index.add(photo)?;
        self.hash_index.add(photo)?;

        assert!(self.all_index.total_count()? == self.hash_index.total_count()?);

        Ok(())
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

    pub fn get_photos_list_from_hashes_list<'s, 'h>(
        &'s mut self,
        hash: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        self.hash_index.get_photos_list_from_hashes_list(hash)
    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        self.all_index.total_count()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PhotoReference {
    pub id: Identifier,
    pub year: i32,
    pub month: u32,
    pub hash: String,
    pub images: Vec<ImageReference>,
    pub shot_time: DateTime<FixedOffset>,
}

impl From<PhotoMeta> for PhotoReference {
    fn from(value: PhotoMeta) -> Self {
        Self {
            year: value.id.year,
            month: value.id.month,
            id: value.id,
            hash: value.original_sha256,
            images: value.images.into_iter().map(Into::into).collect(),
            shot_time: value.shot_time,
        }
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct ImageReference {
    pub id: String,
    pub height: u32,
    pub ext: String,
}

impl From<ImageMeta> for ImageReference {
    fn from(value: ImageMeta) -> Self {
        Self {
            id: value.image_id,
            height: value.height,
            ext: value.extension,
        }
    }
}

pub trait PhotoIndexProvider {
    const INDEX_NAME: &'static str;
    type Entry: serde::Serialize + serde::de::DeserializeOwned + Default + std::fmt::Debug;

    fn add(&mut self, photo: &PhotoMeta) -> anyhow::Result<()>;
    fn total_count(&mut self) -> anyhow::Result<u32>;

    fn load_to_file(&mut self, path: &Path) -> anyhow::Result<Self::Entry> {
        if !path.exists() {
            return self.init(path);
        }

        let file = File::open(path)
            .context(format!("Failed to open a file for the {}", Self::INDEX_NAME))?;

        serde_json::from_reader(file)
            .context(format!("The {} contains invalid content", Self::INDEX_NAME))
    }

    fn init(&mut self, path: &Path) -> anyhow::Result<Self::Entry> {
        let mut file =
            File::create(path).context(format!("Failed to create a file for the {}", Self::INDEX_NAME))?;

        let value = Self::Entry::default();
        serde_json::to_writer_pretty(&mut file, &value)
            .context(format!("Failed to write a empty entry for the {}", Self::INDEX_NAME))?;

        Ok(value)
    }
}

