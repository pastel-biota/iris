mod filter;

use std::collections::HashMap;

use anyhow::Context as _;
use diesel::prelude::*;

use crate::{
    model::{Identifier, PhotoMeta, PhotoVisibility},
    repository::sqlite::filter::select_visible_photos,
};

use crate::infra::{schema::photos, sqlite::SqliteConnection};

pub struct SqlitePhotoIndex {
    db: SqliteConnection,
}

impl SqlitePhotoIndex {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn add_new_photo(&mut self, photo: PhotoMeta) -> anyhow::Result<()> {
        let pool = self.db.pool();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            diesel::dsl::insert_into(photos::table)
                .values((
                    photos::id.eq(photo.id().to_string()),
                    photos::federated_by.eq(photo
                        .origin
                        .federator()
                        .map(|entity_name| entity_name.to_string())),
                    photos::shot_time_unix.eq(photo.shot_time.timestamp()),
                    photos::cached_meta_json.eq(serde_json::to_string(&photo).unwrap()),
                ))
                .execute(&mut conn)
                .context("Failed to update the DB")?;

            Ok(())
        })
        .await
        .context("DB task panicked")?
    }

    pub async fn upsert_photo(&mut self, photo: PhotoMeta) -> anyhow::Result<()> {
        let pool = self.db.pool();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            diesel::dsl::insert_into(photos::table)
                .values((
                    photos::id.eq(photo.id().to_string()),
                    photos::federated_by.eq(photo
                        .origin
                        .federator()
                        .map(|entity_name| entity_name.to_string())),
                    photos::shot_time_unix.eq(photo.shot_time.timestamp()),
                    photos::cached_meta_json.eq(serde_json::to_string(&photo).unwrap()),
                ))
                .on_conflict(photos::id)
                .do_update()
                .set((
                    photos::shot_time_unix.eq(photo.shot_time.timestamp()),
                    photos::cached_meta_json.eq(serde_json::to_string(&photo).unwrap()),
                ))
                .execute(&mut conn)
                .context("Failed to update the DB")?;

            Ok(())
        })
        .await
        .context("DB task panicked")?
    }

    pub async fn get_photo_ref(
        &self,
        visibility: PhotoVisibility,
        photo_id: &Identifier,
    ) -> anyhow::Result<Option<PhotoMeta>> {
        let photo_id = photo_id.to_string();
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, String)>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            select_visible_photos(&visibility)
                .select((photos::id, photos::cached_meta_json))
                .filter(photos::id.eq(photo_id))
                .limit(1)
                .load(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(rows_into_references(rows).into_iter().next())
    }

    pub async fn list_images(
        &self,
        visibility: PhotoVisibility,
        beginning: Option<(i64, &Identifier)>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoMeta>> {
        let pool = self.db.pool();
        let beginning = beginning.map(|(shot_time, id)| (shot_time, id.clone()));

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, String)>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let mut query = select_visible_photos(&visibility);

            if let Some(beginning) = &beginning {
                query = filter::select_after_cursor(query, beginning);
            }

            query
                .select((photos::id, photos::cached_meta_json))
                .limit(size as i64)
                .load(&mut conn)
                .context("Failed on executing the query")
        })
        .await
        .context("DB task panicked")??;

        Ok(rows_into_references(rows))
    }

    pub async fn image_exists_with_hash(&self, hash: &str) -> anyhow::Result<bool> {
        let hash = hash.to_string();
        let pool = self.db.pool();

        let exists = tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            // Safety: hash conflict check must be done globally and
            //         this method exposes minimal information
            #[allow(clippy::disallowed_methods)]
            diesel::select(diesel::dsl::exists(
                photos::table.filter(photos::original_sha256.eq(&hash)),
            ))
            .get_result(&mut conn)
            .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(exists)
    }

    pub fn get_photos_list_from_hashes_list<'s, 'h>(
        &'s mut self,
        _hashes: &'h [String],
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoMeta>> {
        unimplemented!("Unused");
    }

    pub async fn get_photos_list_by_ids_list(
        &self,
        visibility: PhotoVisibility,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<PhotoMeta>> {
        let id_strings = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, String)>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            select_visible_photos(&visibility)
                .filter(photos::id.eq_any(&id_strings))
                .select((photos::id, photos::cached_meta_json))
                .load(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(rows_into_references(rows))
    }

    pub fn delete_photo(&mut self, _photo_id: &Identifier) -> anyhow::Result<()> {
        todo!();
    }

    pub async fn total_count(&self, visibility: PhotoVisibility) -> anyhow::Result<u32> {
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<i64> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            select_visible_photos(&visibility)
                .count()
                .get_result(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(rows.try_into().unwrap())
    }
}

fn rows_into_references(rows: Vec<(String, String)>) -> Vec<PhotoMeta> {
    rows.into_iter()
        .flat_map(|(id, meta_json)| {
            let reference = serde_json::from_str::<PhotoMeta>(&meta_json);

            if let Err(error) = &reference {
                tracing::warn!(
                    "Reference JSON in DB has corruption: for ID {}: {}",
                    id,
                    error
                );
            }
            reference.ok()
        })
        .collect()
}
