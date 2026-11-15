use std::collections::{HashMap, HashSet};

use anyhow::Context;
use chrono::{DateTime, FixedOffset};
use tokio::io::AsyncRead;

use crate::{
    infra::sqlite::SqliteConnection,
    model::{
        EntityName, Identifier, ImageMeta, LocalIdentifier, PhotoMeta, PhotoOrigin,
        PhotoRelatedFile, PhotoRelatedFileRole, PhotoVisibility, Properties,
    },
    repository::{
        config::FederationConfig,
        federated::FederatedPhotoIndex,
        io::{LengthedStream, ScopedPath},
        local::io::PhotoStorageDirectory,
        sqlite::SqlitePhotoIndex,
        types::PhotoFilter,
    },
};

pub const INDEXER_NAME: &'static str = "photo-v2_photo-tag";

pub struct PhotoStorageRegistry {
    pub dir: PhotoStorageDirectory,
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
    pub file_name_hint: String,
}

impl PhotoStorageRegistry {
    pub fn new(
        federation_config: FederationConfig,
        sqlite_connection: SqliteConnection,
        base_dir: &ScopedPath,
    ) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            federated_index: FederatedPhotoIndex::new(federation_config),
            local_index: SqlitePhotoIndex::new(sqlite_connection.clone()),
        }
    }

    pub async fn list_images(
        &self,
        visibility: PhotoVisibility,
        filter: PhotoFilter<'_>,
    ) -> anyhow::Result<(Vec<PhotoMeta>, u32)> {
        self.local_index.list_images(visibility, filter).await
    }

    pub async fn get_image_with_hash(
        &self,
        visibility: PhotoVisibility,
        hash: &str,
    ) -> anyhow::Result<Option<PhotoMeta>> {
        self.local_index.get_image_with_hash(visibility, hash).await
    }

    pub async fn get_image_from_stem(
        &self,
        visibility: PhotoVisibility,
        stem: &str,
    ) -> anyhow::Result<Option<PhotoMeta>> {
        self.local_index.get_image_with_stem(visibility, stem).await
    }

    pub async fn image_exists_with_hash(&self, hash: &str) -> anyhow::Result<bool> {
        self.local_index.image_exists_with_hash(hash).await
    }

    pub fn get_photos_list_by_hashes_list<'s, 'h>(
        &'s mut self,
        hash_to_lookup: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoMeta>> {
        self.local_index
            .get_photos_list_from_hashes_list(hash_to_lookup)
    }

    pub async fn total_count(&self, visibility: PhotoVisibility) -> anyhow::Result<u32> {
        self.local_index.total_count(visibility).await
    }

    pub async fn load_photo(&self, id: &Identifier) -> anyhow::Result<Option<PhotoMeta>> {
        let Some(photo_ref) = self
            .local_index
            .get_photo_ref(PhotoVisibility::All, id)
            .await?
        else {
            return Ok(None);
        };

        match &photo_ref.origin {
            PhotoOrigin::Local(local_id) => Ok(Some(self.dir.load_photo_meta(local_id)?)),
            PhotoOrigin::Federated(remote) => {
                Ok(Some(self.federated_index.get_photos_meta(remote).await?))
            }
        }
    }

    pub async fn sync_image_list(&mut self, name: &EntityName) -> anyhow::Result<()> {
        let photos = self
            .federated_index
            // XXX
            .list_photos(name, Some(1000), None)
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
            PhotoOrigin::Local(local_id) => self.dir.load_image(local_id, image_id, image).await,
            PhotoOrigin::Federated(remote) => self
                .federated_index
                .get_photo_image(remote, image_id)
                .await
                .context("Could not retrieve image from federator"),
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

        self.dir.load_original_image(local_id, original).await
    }

    pub fn update_photo_meta(&mut self, meta: PhotoMeta) -> anyhow::Result<()> {
        self.dir.update_photo_meta(meta)
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
            file_name_hint: Some(meta.file_name_hint),
            tags: vec![],
            files: HashMap::new(),
        };

        self.dir.create_new_photo_meta(meta.clone())?;
        self.local_index.add_new_photo(meta.clone()).await?;

        Ok(meta)
    }

    pub async fn upload_original_image(
        &mut self,
        id: &Identifier,
        file_name: &str,
        file_data: impl AsyncRead,
    ) -> anyhow::Result<PhotoMeta> {
        let Some(photo_ref) = self
            .local_index
            .get_photo_ref(PhotoVisibility::All, id)
            .await?
        else {
            anyhow::bail!("The photo with id {id} is not found");
        };

        let Some(local_id) = photo_ref.origin.local_id() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        let original_file = PhotoRelatedFile {
            file_name: file_name.to_string(),
            role: HashSet::from([PhotoRelatedFileRole::Original]),
        };

        self.dir
            .upload_related_file(local_id, original_file, file_data)
            .await
    }

    pub async fn upload_related_file(
        &mut self,
        photo_id: &Identifier,
        file: PhotoRelatedFile,
        file_data: impl AsyncRead,
    ) -> anyhow::Result<PhotoMeta> {
        let Some(photo_ref) = self
            .local_index
            .get_photo_ref(PhotoVisibility::All, photo_id)
            .await?
        else {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        let Some(local_id) = photo_ref.origin.local_id() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        self.dir
            .upload_related_file(local_id, file, file_data)
            .await
    }

    pub async fn rename_legacy_file(
        &mut self,
        photo_id: &Identifier,
        file_name: &str,
    ) -> anyhow::Result<PhotoMeta> {
        let Some(photo_ref) = self
            .local_index
            .get_photo_ref(PhotoVisibility::All, photo_id)
            .await?
        else {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        let Some(local_id) = photo_ref.origin.local_id() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        let Some(original) = &photo_ref.original else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        self.dir
            .name_legacy_original_file(local_id, &original.extension, &file_name)
            .await
    }

    pub async fn upload_image(
        &mut self,
        photo_id: &Identifier,
        image_id: &str,
        image: &ImageMeta,
        content: &[u8],
    ) -> anyhow::Result<PhotoMeta> {
        let photo = self
            .load_photo(photo_id)
            .await?
            .context("The photo does not exist")?;

        let Some(local_id) = photo.origin.local_id().cloned() else {
            anyhow::bail!("You cannot update the original image of non-local (federated) photo");
        };

        let photo = self
            .dir
            .upload_image(&local_id, image_id, image, content)
            .await?;

        self.local_index.upsert_photo(photo.clone()).await?;

        Ok(photo)
    }

    pub async fn unregister(&mut self, photo_id: &Identifier) -> anyhow::Result<()> {
        if self
            .local_index
            .get_photo_ref(PhotoVisibility::All, photo_id)
            .await?
            .is_none()
        {
            anyhow::bail!("The photo with id {photo_id} is not found");
        };

        self.local_index.delete_photo(photo_id)
    }
}
