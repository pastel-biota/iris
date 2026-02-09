use std::{collections::HashMap, path::Path};

use anyhow::{Context, bail};
use tokio::io::AsyncRead;

use crate::{infra::{io::PhotoStorageDirectory, meta::{ImageMeta, PhotoMeta}}, model::Identifier};

pub struct PhotoStorageRegistry {
    dir: PhotoStorageDirectory,
    loaded_photos: HashMap<Identifier, PhotoMeta>,
}

impl PhotoStorageRegistry {
    pub fn new(base_dir: &Path) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            loaded_photos: HashMap::new(),
        }
    }

    pub fn load_photo(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        if let Some(photo) = self.loaded_photos.get(&id) {
            return Ok(Some(photo.clone()));
        }

        let Some(photo) = self.dir.load_photo_meta(id)? else {
            return Ok(None);
        };

        self.loaded_photos.insert(id.clone(), photo.clone());

        Ok(Some(photo))
    }

    pub async fn load_image(&mut self, id: &Identifier, image: &ImageMeta) -> anyhow::Result<Vec<u8>> {
        self.dir.load_image(id, image).await
    }

    pub fn new_photo(&mut self, meta: PhotoMeta) -> anyhow::Result<()> {
        self.dir.create_new_photo_meta(meta)
    }

    pub async fn upload_image(&mut self, id: &Identifier, image_id: &str, ext: &str, content: impl AsyncRead) -> anyhow::Result<()> {
        let photo = self.dir.load_photo_meta(id)?
            .context("The photo does not exist")?;

        let image = photo.images
            .iter()
            .find(|image| image.image_id == image_id)
            .context("The photo was found, but there is not image defined with that id")?;

        if image.extension != ext {
            bail!("The photo and image was found, but the extension is not right ({} != {})", image.extension, ext);
        }

        self.dir
            .upload_photo(id, image, content)
            .await
    }
}
