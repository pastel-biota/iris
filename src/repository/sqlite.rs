use std::collections::HashMap;

use anyhow::Context as _;
use diesel::{prelude::*, query_builder::BoxedSqlQuery};

use crate::{
    auth::config::Entity, model::{Identifier, ImageMeta, LocalIdentifier, PhotoReference},
};

use crate::infra::{schema::{photos, whitelist}, sqlite::SqliteConnection};

pub struct SqlitePhotoIndex {
    db: SqliteConnection,
}

impl SqlitePhotoIndex {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn add_new_photo(&mut self, photo: PhotoReference) -> anyhow::Result<()> {
        let pool = self.db.pool();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            diesel::dsl::insert_into(photos::table)
                .values((
                    photos::id.eq(photo.id().to_string()),
                    photos::federated_by.eq(photo.origin.federator().map(|entity_name| entity_name.to_string())),
                    photos::original_sha256.eq(&photo.hash),
                    photos::shot_time_unix.eq(photo.shot_time.timestamp()),
                    photos::meta_json.eq(serde_json::to_string(&photo).unwrap())
                ))
                .execute(&mut conn)
                .context("Failed to update the DB")?;

            Ok(())
        })
        .await
        .context("DB task panicked")?
    }

    pub async fn upsert_photo(&mut self, photo: PhotoReference) -> anyhow::Result<()> {
        let pool = self.db.pool();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            diesel::dsl::insert_into(photos::table)
                .values((
                    photos::id.eq(photo.id().to_string()),
                    photos::federated_by.eq(photo.origin.federator().map(|entity_name| entity_name.to_string())),
                    photos::original_sha256.eq(&photo.hash),
                    photos::shot_time_unix.eq(photo.shot_time.timestamp()),
                    photos::meta_json.eq(serde_json::to_string(&photo).unwrap())
                ))
                .on_conflict(photos::id)
                .do_update()
                .set(photos::meta_json.eq(serde_json::to_string(&photo).unwrap()))
                .execute(&mut conn)
                .context("Failed to update the DB")?;

            Ok(())
        })
        .await
        .context("DB task panicked")?
    }

    pub async fn get_photo_ref(
        &self,
        entity: Option<Entity>,
        photo_id: &Identifier,
    ) -> anyhow::Result<Option<PhotoReference>> {
        let restrict_to_entity = restrict_to_entity(entity.as_ref());
        let photo_id = photo_id.to_string();
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, String)>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let mut query = photos::table.into_boxed();

            if let Some(entity_name) = &restrict_to_entity {
                query = query.filter(diesel::dsl::exists(
                    whitelist::table.filter(
                        whitelist::photo_id
                            .eq(photos::id)
                            .and(whitelist::entity.eq(entity_name)),
                    ),
                ));
            }

            query = query.filter(photos::id.eq(photo_id));

            query
                .select((photos::id, photos::meta_json))
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
        entity: Option<&Entity>,
        beginning: Option<&Identifier>,
        size: usize,
    ) -> anyhow::Result<Vec<PhotoReference>> {
        let restrict_to_entity = restrict_to_entity(entity);
        let begin_cursor = beginning.map(|cursor| cursor.to_string());
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, String)>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let mut query = photos::table.into_boxed();

            if let Some(entity_name) = &restrict_to_entity {
                query = query.filter(diesel::dsl::exists(
                    whitelist::table.filter(
                        whitelist::photo_id
                            .eq(photos::id)
                            .and(whitelist::entity.eq(entity_name)),
                    ),
                ));
            }

            if let Some(begin) = &begin_cursor {
                query = query.filter(photos::id.gt(begin));
            }

            let query = query
                .select((photos::id, photos::meta_json))
                .limit(size as i64);

            query
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
    ) -> anyhow::Result<HashMap<&'h str, &'s PhotoReference>> {
        unimplemented!("Unused");
    }

    pub async fn get_photos_list_by_ids_list(
        &self,
        entity: Option<Entity>,
        ids: &[Identifier],
    ) -> anyhow::Result<Vec<PhotoReference>> {
        let restrict_to_entity = restrict_to_entity(entity.as_ref());
        let id_strings = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, String)>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let mut query = photos::table
                .filter(photos::id.eq_any(&id_strings))
                .into_boxed();

            if let Some(entity_name) = &restrict_to_entity {
                query = query.filter(diesel::dsl::exists(
                    whitelist::table.filter(
                        whitelist::photo_id
                            .eq(photos::id)
                            .and(whitelist::entity.eq(entity_name)),
                    ),
                ));
            }

            query
                .select((photos::id, photos::meta_json))
                .load(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(rows_into_references(rows))
    }

    pub fn delete_photo(&mut self, photo_id: &Identifier) -> anyhow::Result<()> {
        todo!();
    }

    pub async fn total_count(&self, entity: Option<&Entity>) -> anyhow::Result<u32> {
        let restrict_to_entity = restrict_to_entity(entity);
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<i64> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let mut query = photos::table.into_boxed();

            if let Some(entity_name) = &restrict_to_entity {
                query = query.filter(diesel::dsl::exists(
                    whitelist::table.filter(
                        whitelist::photo_id
                            .eq(photos::id)
                            .and(whitelist::entity.eq(entity_name)),
                    ),
                ));
            }

            query
                .count()
                .get_result(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(rows.try_into().unwrap())
    }
}

/// `None` means unrestricted access; `Some(name)` restricts to photos whitelisted for that entity.
fn restrict_to_entity(entity: Option<&Entity>) -> Option<String> {
    match entity {
        Some(entity) if !entity.has_full_image_access() => Some(entity.name().to_string()),
        _ => None,
    }
}

fn rows_into_references(rows: Vec<(String, String)>) -> Vec<PhotoReference> {
    rows.into_iter()
        .flat_map(|(id, meta_json)| {
            let reference = serde_json::from_str::<PhotoReference>(&meta_json);

            if let Err(error) = &reference {
                tracing::warn!("Reference JSON in DB has corruption: for ID {}: {}", id, error);
            }
            reference.ok()
        })
        .collect()
}
