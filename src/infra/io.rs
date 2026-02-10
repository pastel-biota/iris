use std::path::{Path, PathBuf};

use anyhow::Context;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt as _},
    pin,
};

use crate::{
    infra::meta::{ImageMeta, PhotoMeta},
    model::Identifier,
};

pub struct PhotoStorageDirectory {
    base_dir: PathBuf,
}

impl PhotoStorageDirectory {
    pub fn new(base_dir: &Path) -> Self {
        PhotoStorageDirectory {
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn load_photo_meta(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        let paths = PathsForPhoto::from_id(&self.base_dir, &id);

        if !self.photo_exists(id) {
            return Ok(None);
        }

        let file_content = std::fs::read(paths.meta())
            .with_context(|| format!("Could not read the metafile for {}", id.to_string()))?;
        let meta =
            serde_json::from_slice::<PhotoMeta>(file_content.as_slice()).with_context(|| {
                format!(
                    "The metafile for {} exists, but is malformed",
                    id.to_string()
                )
            })?;

        Ok(Some(meta))
    }

    pub async fn load_image(
        &mut self,
        photo_id: &Identifier,
        image: &ImageMeta,
    ) -> anyhow::Result<Vec<u8>> {
        let paths = PathsForPhoto::from_id(&self.base_dir, &photo_id);

        let mut image_file = File::open(paths.for_image(&image.image_id, &image.extension))
            .await
            .context("Failed to open image file")?;

        let mut vec = Vec::new();
        image_file
            .read_to_end(&mut vec)
            .await
            .context("Failed to read image file")?;

        Ok(vec)
    }

    pub fn photo_exists(&self, id: &Identifier) -> bool {
        let paths = PathsForPhoto::from_id(&self.base_dir, id);

        paths.meta().exists()
    }

    pub fn create_new_photo_meta(&mut self, meta: PhotoMeta) -> anyhow::Result<()> {
        let paths = PathsForPhoto::from_id(&self.base_dir, &meta.id);

        if paths.meta().exists() {
            anyhow::bail!("The metafile collided ('{}')", &meta.id.to_string());
        }

        if !paths.base_dir.exists() {
            std::fs::create_dir_all(&paths.base_dir).with_context(|| {
                format!(
                    "Could not create directory '{}' for new photo",
                    &paths.base_dir.display()
                )
            })?;
        }

        let meta = serde_json::to_vec_pretty(&meta)
            .context("Could not create the JSON represent for the metafile")?;

        std::fs::write(paths.meta(), meta)
            .context("Could not write the metaafile for new photo")?;

        Ok(())
    }

    pub async fn upload_photo(
        &mut self,
        id: &Identifier,
        image: &ImageMeta,
        image_data: impl AsyncRead,
    ) -> anyhow::Result<()> {
        let paths = PathsForPhoto::from_id(&self.base_dir, id);

        let path = paths.for_image(&image.image_id, &image.extension);

        let result = {
            let mut file = File::create(&path)
                .await
                .context("Failed to open the image file")?;

            pin!(image_data);

            tokio::io::copy(&mut image_data, &mut file).await
        };

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Err(err) = tokio::fs::remove_file(path).await {
                    eprintln!(
                        "warning: The file could not be removed after the failed write: {err}"
                    );
                }
                Err(err).context("The image content could not be written to the file")
            }
        }
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
        self.base_dir
            .join(format!("{}-meta.json", self.id.to_string()))
    }

    pub fn for_image(&self, img_id: &str, ext: &str) -> PathBuf {
        self.base_dir
            .join(format!("{}-{}.{}", self.id.to_string(), img_id, ext))
    }
}
