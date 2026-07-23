use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, FixedOffset};

use crate::{
    auth::config::Entity, infra::sqlite::SqliteConnection, model::{EntityName, Identifier, ImageMeta, LocalIdentifier, PhotoMeta, PhotoOrigin, PhotoReference, Properties}, repository::{
        config::FederationConfig, federated::FederatedPhotoIndex, io::{LengthedStream, ScopedPath}, local::io::PhotoStorageDirectory, sqlite::SqlitePhotoIndex
    }
};

pub struct PhotoStorageRegistry {
    dir: PhotoStorageDirectory,
    pub federated_index: FederatedPhotoIndex,
    pub local_index: SqlitePhotoIndex,
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
        sqlite_connection: SqliteConnection,
        base_dir: &ScopedPath
    ) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            federated_index: FederatedPhotoIndex::new(federation_config),
            local_index: SqlitePhotoIndex::new(sqlite_connection),
        }
    }

    pub async fn list_images(
        &self,
        entity: Option<&Entity>,
        beginning: Option<&Identifier>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        self.local_index.list_images(entity, beginning, size).await
    }

    pub async fn get_photos_list_by_id_list(
        &self,
        entity: Option<Entity>,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<PhotoReference>> {
        self.local_index.get_photos_list_by_ids_list(entity, ids).await
    }

    pub async fn image_exists_with_hash(&self, hash: &str) -> anyhow::Result<bool> {
        self.local_index.image_exists_with_hash(hash).await
    }

    pub fn get_photos_list_by_hashes_list<'s, 'h>(
        &'s mut self,
        hash_to_lookup: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        self.local_index.get_photos_list_from_hashes_list(hash_to_lookup)

    }

    pub async fn total_count(&self, entity: Option<&Entity>) -> anyhow::Result<u32> {
        self.local_index.total_count(entity).await
    }

    pub async fn load_photo(&self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        let Some(photo_ref) = self.local_index.get_photo_ref(None, id).await? else {
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

    pub async fn load_photo_ref(&self, id: &Identifier) -> anyhow::Result<Option<PhotoReference>> {
        let Some(photo_ref) = self.local_index.get_photo_ref(None, id).await? else {
            return Ok(None);
        };

        Ok(Some(photo_ref))
    }

    pub async fn sync_image_list(&mut self, name: &EntityName) -> anyhow::Result<()> {
        let photos = self.federated_index.list_photos(name, None, None)
            .await
            .context("Failed to retrieve the photos from federated node")?;

        for photo in photos.photos {
            self.local_index.upsert_photo(photo).await?;
        }

        Ok(())
    }

    /// Loads an image given the photo's already-resolved origin, avoiding a
    /// redundant photo-ref lookup when the caller has already loaded the photo.
    pub async fn load_image(
        &self,
        origin: &PhotoOrigin,
        image_id: &str,
        image: &ImageMeta,
    ) -> anyhow::Result<LengthedStream> {
        match origin {
            PhotoOrigin::Local(local_id) => {
                self.dir.load_image(local_id, image_id, image).await
            },
            PhotoOrigin::Federated(remote) => {
                self.federated_index.get_photo_image(remote, image_id).await
                    .context("Could not retrieve image from federator")
            },
        }
    }

    pub async fn load_original_image(&self, photo_id: &Identifier) -> anyhow::Result<Vec<u8>> {
        let photo = self
            .load_photo(photo_id)
            .await?
            .context("The photo does not exist")?;

        let Some(local_id) = photo.origin.local_id() else {
            anyhow::bail!("You cannot retrieve the original image of non-local (federated) photo");
        };

        let Some(original) = &photo.original else {
            anyhow::bail!("This photo's original image is not available");
        };

        self.dir
            .load_original_image(local_id, original)
            .await
    }

    pub async fn new_photo(&mut self, meta: NewPhotoParam) -> anyhow::Result<PhotoMeta> {
        let meta = PhotoMeta {
            origin: PhotoOrigin::Local(LocalIdentifier(meta.id)),
            original: Some(meta.original),
            images: HashMap::new(),
            original_sha256: meta.original_sha256,
            properties: meta.properties,
            shot_time: meta.shot_time,
            representative_rgb: meta.representative_rgb,
            tags: HashMap::new(),
        };

        self.dir.create_new_photo_meta(meta.clone())?;
        self.local_index.add_new_photo(meta.clone().into()).await?;

        Ok(meta)
    }

    pub async fn upload_original_image(
        &mut self,
        id: &Identifier,
        ext: &str,
        content: &[u8],
    ) -> anyhow::Result<()> {
        let Some(photo_ref) = self.local_index.get_photo_ref(None, id).await? else {
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
        let mut photo = self
            .load_photo(photo_id)
            .await?
            .context("The photo does not exist")?;

        let Some(local_id) = photo.origin.local_id().cloned() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        let meta = self
            .dir
            .upload_image(&local_id, image_id, image, content)
            .await?;

        photo.images.insert(image_id.to_string(), image.clone());
        self.local_index.upsert_photo(photo.into()).await?;

        Ok(meta)
    }

    pub async fn unregister(
        &mut self,
        photo_id: &Identifier,
    ) -> anyhow::Result<()> {
        if self.local_index.get_photo_ref(None, photo_id).await?.is_none() {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        self.local_index.delete_photo(photo_id)
    }

    pub async fn insert_photo_reference(
        &mut self,
        photo: PhotoReference,
    ) -> anyhow::Result<Option<()>> {
        if self.load_photo_ref(photo.id()).await?.is_some() {
            return Ok(None);
        }

        if let Some(local_id) = photo.origin.local_id() && !self.dir.photo_exists(local_id) {
            anyhow::bail!("This photo has its ownership at the local, but the photo does not exist");
        }

        self.local_index.add_new_photo(photo.into()).await?;

        Ok(Some(()))
    }
}
