use std::{collections::HashMap, io::Read, path::{Path, PathBuf}};

use anyhow::{Context, bail};
use tokio::io::AsyncRead;

use crate::{infra::{io::PhotoStorageDirectory, meta::PhotoMeta}, model::Identifier};

pub struct PhotoStorageRegistry {
    dir: PhotoStorageDirectory,
    loaded_photos: HashMap<Identifier, PhotoMeta>
}

impl PhotoStorageRegistry {
    pub fn new(base_dir: &Path) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            loaded_photos: HashMap::new()
        }
    }

    pub fn load_image(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        if let Some(photo) = self.loaded_photos.get(&id) {
            return Ok(Some(photo.clone()));
        }

        let Some(photo) = self.dir.load_image_meta(id)? else {
            return Ok(None);
        };

        self.loaded_photos.insert(id.clone(), photo.clone());

        Ok(Some(photo))
    }

    pub fn new_photo(&mut self, meta: PhotoMeta) -> anyhow::Result<()> {
        self.dir.create_new_photo_meta(meta)
    }

    pub async fn upload_image(&mut self, id: &Identifier, image_id: &str, ext: &str, content: impl AsyncRead) -> anyhow::Result<()> {
        let photo = self.dir.load_image_meta(id)?
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

struct PathsForPhoto {
    base_dir: PathBuf,
    id: Identifier,
}

impl PathsForPhoto {
    pub fn from_id(base_dir: &Path, id: &Identifier) -> Self {
        PathsForPhoto {
            base_dir: base_dir
                .join(format!("{:04}", id.year))
                .join(format!("{:02}", id.month)),
            id: id.clone(),
        }
    }

    pub fn meta(&self) -> PathBuf {
        self.base_dir.join(format!("{}-meta.json", self.id.to_string()))
    }

    pub fn for_image(&self, img_id: &str, ext: &str) -> PathBuf {
        self.base_dir.join(format!("{}-{}.{}", self.id.to_string(), img_id, ext))
    }
}

