use std::{collections::HashMap, path::Path};

use anyhow::{Context, bail};

use crate::{
    infra::{
        io::PhotoStorageDirectory,
        photo_index::{PhotoIndex, PhotoReference},
    },
    model::{Identifier, ImageMeta, PhotoMeta},
};

pub struct PhotoStorageRegistry {
    dir: PhotoStorageDirectory,
    index: PhotoIndex,
    loaded_photos: HashMap<Identifier, PhotoMeta>,
}

impl PhotoStorageRegistry {
    pub fn new(base_dir: &Path) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            index: PhotoIndex::new(base_dir),
            loaded_photos: HashMap::new(),
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
        if let Some(photo) = self.loaded_photos.get(id) {
            return Ok(Some(photo.clone()));
        }

        let Some(photo) = self.dir.load_photo_meta(id)? else {
            return Ok(None);
        };

        self.loaded_photos.insert(id.clone(), photo.clone());

        Ok(Some(photo))
    }

    pub async fn load_image(
        &mut self,
        id: &Identifier,
        image: &ImageMeta,
    ) -> anyhow::Result<Vec<u8>> {
        self.dir.load_image(id, image).await
    }

    pub fn new_photo(&mut self, meta: &PhotoMeta) -> anyhow::Result<()> {
        self.dir.create_new_photo_meta(meta.clone())?;
        self.index.add_new_image(meta)?;

        Ok(())
    }

    pub async fn upload_original_image(
        &mut self,
        id: &Identifier,
        ext: &str,
        content: &[u8],
    ) -> anyhow::Result<()> {
        self
            .dir
            .load_photo_meta(id)?
            .context("The photo does not exist")?;
        self.dir.upload_original_photo(id, ext, content).await?;

        Ok(())
    }

    pub async fn upload_image(
        &mut self,
        id: &Identifier,
        image_id: &str,
        ext: &str,
        content: &[u8],
    ) -> anyhow::Result<ImageMeta> {
        let photo = self
            .dir
            .load_photo_meta(id)?
            .context("The photo does not exist")?;

        let image = photo
            .images
            .iter()
            .find(|image| image.image_id == image_id)
            .context("The photo was found, but there is not image defined with that id")?;

        if image.extension != ext {
            bail!(
                "The photo and image was found, but the extension is not right ({} != {})",
                image.extension,
                ext
            );
        }

        self.dir.upload_photo(id, image, content).await?;

        Ok(image.clone())
    }
}
