use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, FixedOffset};

use crate::{
    infra::sqlite::SqliteConnection,
    model::{
        EntityName, Identifier, ImageMeta, LocalIdentifier, PhotoMeta, PhotoOrigin,
        PhotoVisibility, Properties,
    },
    repository::{
        config::FederationConfig,
        federated::FederatedPhotoIndex,
        indexer::SqliteIndexer,
        io::{LengthedStream, ScopedPath},
        local::io::PhotoStorageDirectory,
        sqlite::SqlitePhotoIndex,
    },
};

pub const INDEXER_NAME: &'static str = "photo-v2";

pub struct PhotoStorageRegistry {
    pub dir: PhotoStorageDirectory,
    pub federated_index: FederatedPhotoIndex,
    pub sqlite_indexer: SqliteIndexer,
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
        base_dir: &ScopedPath,
    ) -> Self {
        PhotoStorageRegistry {
            dir: PhotoStorageDirectory::new(base_dir),
            federated_index: FederatedPhotoIndex::new(federation_config),
            local_index: SqlitePhotoIndex::new(sqlite_connection.clone()),
            sqlite_indexer: SqliteIndexer::new(sqlite_connection),
        }
    }

    pub async fn sync_index(&mut self) {
        tracing::info!("Starting to re-index");

        let mut re_indexed = 0usize;
        let mut skipped = 0usize;
        let mut errored = 0usize;

        for photo in self.dir.scan_photo_jsons() {
            let photo = match photo {
                Ok(photo) => photo,
                Err(err) => {
                    tracing::warn!("Error during scanning the photo: {}", err);
                    errored += 1;
                    continue;
                }
            };

            let need_reindex = self
                .sqlite_indexer
                .need_reindex(
                    INDEXER_NAME,
                    &format!("photo-{}", photo.id),
                    &photo.path,
                    &photo.metadata,
                )
                .await;

            let need_reindex = match need_reindex {
                Ok(need_reindex) => need_reindex,
                Err(err) => {
                    tracing::warn!(
                        photo_id = photo.id.to_string(),
                        "Error during checking the photo needs re-indexing: {}",
                        err
                    );
                    errored += 1;
                    continue;
                }
            };

            if !need_reindex {
                skipped += 1;
                continue;
            }

            let photo_meta = match self.dir.load_photo_meta(&photo.id) {
                Ok(photo_meta) => photo_meta,
                Err(err) => {
                    tracing::warn!(
                        photo_id = photo.id.to_string(),
                        "Error during reading the PhotoMeta: {}",
                        err
                    );
                    errored += 1;
                    continue;
                }
            };

            self.local_index.upsert_photo(photo_meta).await.unwrap();
            self.sqlite_indexer
                .record_index(
                    INDEXER_NAME,
                    &format!("photo-{}", photo.id),
                    &photo.path,
                    &photo.metadata,
                )
                .await
                .unwrap();

            re_indexed += 1;
        }

        tracing::info!(re_indexed, skipped, errored, "Re-indexed photos");
    }

    pub async fn list_images(
        &self,
        visibility: PhotoVisibility,
        beginning: Option<(i64, &Identifier)>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoMeta>> {
        self.local_index
            .list_images(visibility, beginning, size)
            .await
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
            PhotoOrigin::Local(local_id) => {
                let static_meta = self.dir.load_photo_meta(local_id)?;

                Ok(Some(PhotoMeta {
                    origin: static_meta.origin,
                    original: static_meta.original,
                    original_sha256: static_meta.original_sha256,
                    properties: static_meta.properties,
                    shot_time: static_meta.shot_time,
                    representative_rgb: static_meta.representative_rgb,
                    images: photo_ref.images,
                    tags: photo_ref.tags,
                }))
            }
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

    pub async fn new_photo(&mut self, meta: NewPhotoParam) -> anyhow::Result<PhotoMeta> {
        let meta = PhotoMeta {
            origin: PhotoOrigin::Local(LocalIdentifier(meta.id)),
            original: Some(meta.original),
            images: HashMap::new(),
            original_sha256: meta.original_sha256,
            properties: meta.properties,
            shot_time: meta.shot_time,
            representative_rgb: meta.representative_rgb,
            tags: vec![],
        };

        self.dir.create_new_photo_meta(meta.clone())?;
        self.local_index.add_new_photo(meta.clone()).await?;

        Ok(meta)
    }

    pub async fn upload_original_image(
        &mut self,
        id: &Identifier,
        ext: &str,
        content: &[u8],
    ) -> anyhow::Result<()> {
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

        self.dir
            .upload_original_image(local_id, ext, content)
            .await?;

        Ok(())
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

        let static_meta = self
            .dir
            .upload_image(&local_id, image_id, image, content)
            .await?;

        let meta = PhotoMeta {
            origin: static_meta.origin,
            original: static_meta.original,
            images: static_meta.images,
            original_sha256: static_meta.original_sha256,
            properties: static_meta.properties,
            shot_time: static_meta.shot_time,
            representative_rgb: static_meta.representative_rgb,
            tags: photo.tags,
        };

        self.local_index.upsert_photo(meta.clone()).await?;

        Ok(meta)
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
