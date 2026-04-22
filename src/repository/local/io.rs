use anyhow::{Context, bail};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _},
    pin,
};

use crate::{model::{ImageMeta, LocalIdentifier, PhotoMeta}, repository::io::ScopedPath};

pub struct PhotoStorageDirectory {
    base_dir: ScopedPath,
}

impl PhotoStorageDirectory {
    pub fn new(base_dir: &ScopedPath) -> Self {
        PhotoStorageDirectory {
            base_dir: base_dir.clone(),
        }
    }

    pub fn load_photo_meta(&mut self, id: &LocalIdentifier) -> anyhow::Result<Option<PhotoMeta>> {
        let paths = PathsForPhoto::from_id(&self.base_dir, id);

        if !self.photo_exists(id) {
            return Ok(None);
        }

        let file_content = paths.meta().read_binary()
            .with_context(|| format!("Could not read the metafile for {id}"))?;

        let meta = serde_json::from_slice::<PhotoMeta>(file_content.as_slice())
            .with_context(|| format!("The metafile for {id} exists, but is malformed"))?;

        Ok(Some(meta))
    }

    pub async fn load_image(
        &mut self,
        photo_id: &LocalIdentifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<Vec<u8>> {
        let paths = PathsForPhoto::from_id(&self.base_dir, photo_id);

        let mut image_file = paths.for_image(image_id, &image.extension)
            .open_file()
            .await
            .context("Failed to open image file")?;

        let mut vec = Vec::new();
        image_file
            .read_to_end(&mut vec)
            .await
            .context("Failed to read image file")?;

        Ok(vec)
    }

    pub async fn load_original_image(
        &mut self,
        photo_id: &LocalIdentifier,
        image: &ImageMeta,
    ) -> anyhow::Result<Vec<u8>> {
        let paths = PathsForPhoto::from_id(&self.base_dir, photo_id);

        let mut image_file = paths.for_original_image(&image.extension)
            .open_file()
            .await
            .context("Failed to open image file")?;

        let mut vec = Vec::new();
        image_file
            .read_to_end(&mut vec)
            .await
            .context("Failed to read image file")?;

        Ok(vec)
    }

    pub fn photo_exists(&self, id: &LocalIdentifier) -> bool {
        let paths = PathsForPhoto::from_id(&self.base_dir, id);

        paths.meta().exists()
    }

    pub fn create_new_photo_meta(&mut self, meta: PhotoMeta) -> anyhow::Result<()> {
        let Some(id) = meta.origin.local_id() else {
            bail!("The PhotoMeta is not local to this instance");
        };

        let paths = PathsForPhoto::from_id(&self.base_dir, id);

        if paths.meta().exists() {
            anyhow::bail!("The metafile collided ('{}')", id);
        }

        if !paths.base_dir.exists() {
            paths.base_dir.create_dir_all().with_context(|| {
                format!(
                    "Could not create directory '{}' for new photo",
                    &paths.base_dir.display()
                )
            })?;
        }

        let meta = serde_json::to_vec_pretty(&meta)
            .context("Could not create the JSON represent for the metafile")?;

        paths.meta().write(meta)
            .context("Could not write the metaafile for new photo")?;

        Ok(())
    }

    pub async fn upload_image(
        &mut self,
        photo_id: &LocalIdentifier,
        image_id: &str,
        image: &ImageMeta,
        image_data: impl AsyncRead,
    ) -> anyhow::Result<PhotoMeta> {
        let paths = PathsForPhoto::from_id(&self.base_dir, photo_id);

        let mut meta = self
            .load_photo_meta(photo_id)?
            .context("The photo was not found")?;

        let path = paths.for_image(image_id, &image.extension);
        self.write_image(&path, image_data).await?;

        meta.images.insert(image_id.to_string(), image.clone());

        let meta_json = serde_json::to_vec_pretty(&meta)
            .context("Could not create the JSON represent for the metafile")?;

        paths.meta().write(meta_json)
            .context("Could not write the metaafile for new photo")?;

        Ok(meta)
    }

    pub async fn upload_original_image(
        &mut self,
        id: &LocalIdentifier,
        extension: &str,
        image_data: impl AsyncRead,
    ) -> anyhow::Result<()> {
        let paths = PathsForPhoto::from_id(&self.base_dir, id);
        let path = paths.for_original_image(extension);

        self.write_image(&path, image_data).await
    }

    async fn write_image(&self, path: &ScopedPath, image_data: impl AsyncRead) -> anyhow::Result<()> {
        let result = {
            path.parent().unwrap().create_dir_all()?;
            let mut file = path.create_file()
                .await
                .context("Failed to open the image file")?;

            pin!(image_data);

            tokio::io::copy(&mut image_data, &mut file).await
        };

        match result {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Err(err) = path.remove_file().await {
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
    base_dir: ScopedPath,
    id: LocalIdentifier,
}

impl PathsForPhoto {
    pub fn from_id(base_dir: &ScopedPath, id: &LocalIdentifier) -> Self {
        PathsForPhoto {
            base_dir: base_dir
                .join(format!("{:04}", id.year))
                .join(format!("{:02}", id.month)),
            id: id.clone(),
        }
    }

    pub fn meta(&self) -> ScopedPath {
        self.base_dir.join(format!("{}-meta.json", self.id))
    }

    pub fn for_image_dir(&self) -> ScopedPath {
        self.base_dir.join(self.id.to_string())
    }

    pub fn for_image(&self, img_id: &str, ext: &str) -> ScopedPath {
        self.for_image_dir()
            .join(format!("{}-{}.{}", self.id, img_id, ext))
    }

    pub fn for_original_image(&self, ext: &str) -> ScopedPath {
        self.base_dir.join(format!("{}.{}", self.id, ext))
    }
}
