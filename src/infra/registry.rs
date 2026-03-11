use std::{collections::HashMap, path::Path};

use anyhow::Context;
use chrono::{DateTime, FixedOffset};

use crate::{
    infra::{
        io::PhotoStorageDirectory,
        photo_index::{PhotoIndex, PhotoReference},
    },
    model::{Identifier, ImageMeta, PhotoMeta, Properties},
};

pub struct PhotoStorageRegistry {
    dir: PhotoStorageDirectory,
    index: PhotoIndex,
}

pub struct NewPhotoParam {
    pub id: Identifier,
    pub original: ImageMeta,
    pub original_sha256: String,
    pub properties: Properties,
    pub shot_time: DateTime<FixedOffset>,
    pub representative_rgb: [u8; 3],
}

impl PhotoStorageRegistry {
    pub fn new(base_dir: &Path) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            index: PhotoIndex::new(base_dir),
        }
    }

    pub fn list_images(
        &mut self,
        beginning: Option<&Identifier>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        self.index.list_images(beginning, size)
    }

    pub fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool> {
        self.index.image_exists_with_hash(hash)
    }

    pub fn get_photos_list_by_hashes_list<'s, 'h>(
        &'s mut self,
        hash_to_lookup: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        self.index.get_photos_list_from_hashes_list(hash_to_lookup)
    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        self.index.total_count()
    }

    pub fn load_photo(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        let Some(photo) = self.dir.load_photo_meta(id)? else {
            return Ok(None);
        };

        Ok(Some(photo))
    }

    pub async fn load_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<Vec<u8>> {
        self.dir.load_image(photo_id, image_id, image).await
    }

    pub async fn load_original_image(
        &mut self,
        photo_id: &Identifier,
    ) -> anyhow::Result<Vec<u8>> {
        let photo = self
            .load_photo(photo_id)?
            .context("The photo does not exist")?;
        self.dir.load_original_image(photo_id, &photo.original).await
    }

    pub fn new_photo(&mut self, meta: NewPhotoParam) -> anyhow::Result<PhotoMeta> {
        let meta = PhotoMeta {
            id: meta.id,
            original: meta.original,
            images: HashMap::new(),
            original_sha256: meta.original_sha256,
            properties: meta.properties,
            shot_time: meta.shot_time,
            representative_rgb: meta.representative_rgb,
        };

        self.dir.create_new_photo_meta(meta.clone())?;
        self.index.add_new_photo(&meta)?;

        Ok(meta)
    }

    pub async fn upload_original_image(
        &mut self,
        id: &Identifier,
        ext: &str,
        content: &[u8],
    ) -> anyhow::Result<()> {
        self
            .load_photo(id)?
            .context("The photo does not exist")?;
        self.dir.upload_original_image(id, ext, content).await?;

        Ok(())
    }

    pub async fn upload_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
        content: &[u8],
    ) -> anyhow::Result<PhotoMeta> {
        let mut photo = self
            .load_photo(photo_id)?
            .context("The photo does not exist")?;
        let meta = self.dir.upload_image(photo_id, image_id, image, content).await?;

        photo.images.insert(image_id.to_string(), image.clone());
        self.index.add_new_image(photo_id, image_id, image)?;

        Ok(meta)
    }
}
