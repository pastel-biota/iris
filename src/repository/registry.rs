use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, FixedOffset};

use crate::{
    model::{EntityName, Identifier, ImageMeta, LocalIdentifier, PhotoMeta, PhotoOrigin, PhotoReference, Properties}, repository::{
        config::FederationConfig, federated::FederatedPhotoIndex, io::{LengthedStream, ScopedPath}, local::io::PhotoStorageDirectory, photo_index::PhotoIndex
    }
};

pub struct PhotoStorageRegistry {
    dir: PhotoStorageDirectory,
    pub federated_index: FederatedPhotoIndex,
    local_index: PhotoIndex,
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
    pub fn new(
        federation_config: FederationConfig,
        base_dir: &ScopedPath
    ) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            federated_index: FederatedPhotoIndex::new(federation_config),
            local_index: PhotoIndex::new(base_dir),
        }
    }

    pub fn list_images(
        &mut self,
        beginning: Option<&Identifier>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        self.local_index.list_images(beginning, size)
    }

    pub fn get_photos_list_by_id_list<'s>(
        &'s mut self,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<&'s PhotoReference>> {
        self.local_index.get_photos_list_by_ids_list(ids)
    }

    pub fn image_exists_with_hash(&mut self, hash: &str) -> anyhow::Result<bool> {
        self.local_index.image_exists_with_hash(hash)
    }

    pub fn get_photos_list_by_hashes_list<'s, 'h>(
        &'s mut self,
        hash_to_lookup: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        self.local_index.get_photos_list_from_hashes_list(hash_to_lookup)

    }

    pub fn total_count(&mut self) -> anyhow::Result<u32> {
        self.local_index.total_count()
    }

    pub async fn load_photo(&mut self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        let Some(photo_ref) = self.local_index.get_photo_ref(id)? else {
            return Ok(None);
        };

        match &photo_ref.origin {
            PhotoOrigin::Local(local_id) => {
                Ok(Some(self.dir.load_photo_meta(local_id)?))
            },
            PhotoOrigin::Federated(remote) => {
                Ok(Some(self.federated_index.get_photos_meta(remote).await?))
            },
        }
    }

    pub async fn sync_image_list(&mut self, name: &EntityName) -> anyhow::Result<()> {
        let photos = self.federated_index.list_photos(name, None, None)
            .await
            .context("Failed to retrieve the photos from federated node")?;

        for photo in photos.photos {
            self.local_index.add_new_photo(&photo)?;
        }

        Ok(())
    }

    pub async fn load_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<LengthedStream> {
        let Some(photo_ref) = self.local_index.get_photo_ref(photo_id)? else {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        match &photo_ref.origin {
            PhotoOrigin::Local(local_id) => {
                self.dir.load_image(local_id, image_id, image).await
            },
            PhotoOrigin::Federated(remote) => {
                self.federated_index.get_photo_image(remote, image_id).await
                    .context("Could not retrieve image from federator")
            },
        }
    }

    pub async fn load_original_image(&mut self, photo_id: &Identifier) -> anyhow::Result<Vec<u8>> {
        let Some(photo_ref) = self.local_index.get_photo_ref(photo_id)? else {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        let Some(local_id) = photo_ref.origin.local_id().cloned() else {
            anyhow::bail!("You cannot retrieve the original image of non-local (federated) photo");
        };

        let photo = self
            .load_photo(photo_id)
            .await?
            .context("The photo does not exist")?;

        let Some(original) = &photo.original else {
            anyhow::bail!("This photo's original image is not available");
        };

        self.dir
            .load_original_image(&local_id, original)
            .await
    }

    pub fn new_photo(&mut self, meta: NewPhotoParam) -> anyhow::Result<PhotoMeta> {
        let meta = PhotoMeta {
            origin: PhotoOrigin::Local(LocalIdentifier(meta.id)),
            original: Some(meta.original),
            images: HashMap::new(),
            original_sha256: meta.original_sha256,
            properties: meta.properties,
            shot_time: meta.shot_time,
            representative_rgb: meta.representative_rgb,
        };

        self.dir.create_new_photo_meta(meta.clone())?;
        self.local_index.add_new_photo(&meta.clone().into())?;

        Ok(meta)
    }

    pub async fn upload_original_image(
        &mut self,
        id: &Identifier,
        ext: &str,
        content: &[u8],
    ) -> anyhow::Result<()> {
        let Some(photo_ref) = self.local_index.get_photo_ref(id)? else {
            anyhow::bail!("The photo with id {id} is not found");
        };

        let Some(local_id) = photo_ref.origin.local_id() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        self.dir.upload_original_image(local_id, ext, content).await?;

        Ok(())
    }

    pub async fn upload_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
        content: &[u8],
    ) -> anyhow::Result<PhotoMeta> {
        let Some(photo_ref) = self.local_index.get_photo_ref(photo_id)? else {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        let Some(local_id) = photo_ref.origin.local_id().cloned() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        let mut photo = self
            .load_photo(photo_id)
            .await?
            .context("The photo does not exist")?;
        let meta = self
            .dir
            .upload_image(&local_id, image_id, image, content)
            .await?;

        photo.images.insert(image_id.to_string(), image.clone());
        self.local_index.add_new_image(photo_id, image_id, image)?;

        Ok(meta)
    }

    pub fn unregister(
        &mut self,
        photo_id: &Identifier,
    ) -> anyhow::Result<()> {
        if self.local_index.get_photo_ref(photo_id)?.is_none() {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        self.local_index.delete_photo(photo_id)
    }
}
