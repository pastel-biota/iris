mod filter;
mod meta;

use std::collections::HashMap;

use anyhow::Context as _;
use diesel::prelude::*;

use crate::{
    infra::sqlite::ConnectionPool,
    model::{Identifier, PhotoMeta, PhotoVisibility},
    repository::{
        sqlite::{
            filter::{PhotoQuery, select_visible_photos},
            meta::{PhotoTableRow, rows_into_reference},
        },
        types::PhotoFilter,
    },
};

use crate::infra::{
    schema::{photo_tag, photos},
    sqlite::SqliteConnection,
};

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

            conn.immediate_transaction(|tx| -> anyhow::Result<()> {
                let id = photo.id().to_string();

                diesel::dsl::insert_into(photos::table)
                    .values((
                        photos::id.eq(&id),
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
                    .execute(tx)
                    .context("failed to update the db")?;

                diesel::dsl::delete(photo_tag::table)
                    .filter(photo_tag::photo_id.eq(&id))
                    .execute(tx)
                    .context("Failed to update the DB")?;

                if !photo.tags.is_empty() {
                    diesel::dsl::insert_into(photo_tag::table)
                        .values(
                            photo
                                .tags
                                .iter()
                                .map(|tag| {
                                    (
                                        photo_tag::photo_id.eq(&id),
                                        photo_tag::tag_identifier.eq(tag.to_string()),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                        .execute(tx)
                        .context("Failed to update the DB")?;
                }

                Ok(())
            })?;

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
        let mut pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<PhotoMeta>> {
            let query = select_visible_photos(&visibility)
                .limit(1)
                .filter(photos::id.eq(photo_id));

            query_photo_table(&mut pool, query).map(|photo| photo.into_iter().next())
        })
        .await
        .context("DB task panicked")??;

        Ok(rows)
    }

    pub async fn list_images(
        &self,
        visibility: PhotoVisibility,
        filter: PhotoFilter<'_>,
    ) -> anyhow::Result<(Vec<PhotoMeta>, u32)> {
        let PhotoFilter {
            beginning,
            size,
            tag,
        } = filter;

        let mut pool = self.db.pool();
        let beginning = beginning.map(|(shot_time, id)| (shot_time, id.clone()));
        let tag = tag.map(|tags| tags.to_vec());

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<(Vec<PhotoMeta>, u32)> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let build_query = || {
                let mut query = select_visible_photos(&visibility);

                if let Some(tag) = &tag {
                    query = filter::select_by_tag(query, tag.iter());
                }

                query
            };

            let total_count: i64 = build_query()
                .count()
                .get_result(&mut conn)
                .context("Failed on executing the query")?;

            let mut query = build_query().limit(size as i64);

            if let Some(beginning) = &beginning {
                query = filter::select_after_cursor(query, beginning);
            }

            Ok((
                query_photo_table(&mut pool, query)?,
                total_count.try_into().unwrap(),
            ))
        })
        .await
        .context("DB task panicked")??;

        Ok(rows)
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

fn query_photo_table<'a>(
    pool: &mut ConnectionPool,
    query: PhotoQuery<'a>,
) -> anyhow::Result<Vec<PhotoMeta>> {
    let mut conn = pool.get().context("Failed to acquire DB connection")?;

    let rows = query
        .select(PhotoTableRow::as_select())
        .load(&mut conn)
        .context("Failed on executing the query")?;

    Ok(rows_into_reference(rows))
}
