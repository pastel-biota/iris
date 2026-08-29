use std::{borrow::Cow, fs::Metadata, os::unix::fs::MetadataExt};

use anyhow::Context as _;
use diesel::{
    ExpressionMethods, QueryDsl, RunQueryDsl as _, Selectable, SelectableHelper,
    deserialize::Queryable,
};

use crate::{
    infra::{schema, sqlite::SqliteConnection},
    repository::io::ScopedPath,
};

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = schema::indexed_file)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct IndexedFileRow {
    pub index_id: String,
    pub source_file: String,
    pub source_mtime_ns: i64,
    pub source_size: i64,
    pub projector_version: String,
}

pub struct IndexHistory {
    db: SqliteConnection,
}

impl IndexHistory {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn need_reindex(
        &self,
        indexer: &'static str,
        index_id: &str,
        path: &ScopedPath,
        metadata: &Metadata,
    ) -> anyhow::Result<bool> {
        let pool = self.db.pool();

        let index = tokio::task::spawn_blocking({
            let index_id = index_id.to_string();
            move || -> anyhow::Result<Option<IndexedFileRow>> {
                let mut conn = pool.get().context("Failed to acquire DB connection")?;

                schema::indexed_file::table
                    .filter(schema::indexed_file::index_id.eq(index_id))
                    .select(IndexedFileRow::as_select())
                    .limit(1)
                    .load(&mut conn)
                    .context("Failed on executing the query")
                    .map(|rows| rows.into_iter().next())
            }
        })
        .await
        .context("DB task panicked")??;

        let Some(index) = index else {
            tracing::debug!(
                index_id,
                reindexing = true,
                "never indexed, this needs re-indexation"
            );
            return Ok(true);
        };

        let path = path.use_path().to_string_lossy();
        if let Cow::Owned(path) = &path {
            tracing::warn!(
                index_id,
                file = path,
                "This index contains non UTF-8 characters, and might cause issues"
            );
        }

        if index.source_file != path {
            tracing::debug!(
                index_id,
                mtime_ns = index.source_mtime_ns,
                last_index_path = index.source_file,
                current_source_path = path.to_string(),
                reindexing = true,
                "index source's mtime_ns has changed, this needs re-indexation"
            );
            return Ok(true);
        }

        if index.source_mtime_ns != metadata.mtime_nsec() {
            tracing::debug!(
                index_id,
                last_index_mtime_ns = index.source_mtime_ns,
                current_source_mtime_ns = metadata.mtime_nsec(),
                reindexing = true,
                "index source's mtime_ns has changed, this needs re-indexation"
            );
            return Ok(true);
        }

        let size: u64 = index.source_size.try_into()
            .expect("The file size needs to be represented in i64/u64, but the stored value was outside of the range");

        if size != metadata.len() {
            tracing::debug!(
                index_id,
                mtime_ns = index.source_mtime_ns,
                last_index_size = index.source_size,
                current_source_size = metadata.len(),
                reindexing = true,
                "index source's mtime_ns has changed, this needs re-indexation"
            );
            return Ok(true);
        }

        if index.projector_version != indexer {
            tracing::debug!(
                index_id,
                mtime_ns = index.source_mtime_ns,
                last_indexer = index.projector_version,
                current_indexer = indexer,
                reindexing = true,
                "the file didn't change, but the indexer changed, this needs re-indexation"
            );
            return Ok(true);
        }

        tracing::trace!(
            index_id,
            last_index_mtime_ns = index.source_mtime_ns,
            last_index_size = index.source_size,
            last_index_path = index.source_file,
            last_indexer = index.projector_version,
            current_source_mtime_ns = metadata.mtime_nsec(),
            current_source_size = metadata.len(),
            current_source_path = path.to_string(),
            current_indexer = indexer,
            reindexing = false,
            "no need of re-indexation for this index"
        );

        return Ok(false);
    }

    pub async fn record_index(
        &self,
        indexer: &'static str,
        index_id: &str,
        path: &ScopedPath,
        metadata: &Metadata,
    ) -> anyhow::Result<()> {
        let pool = self.db.pool();

        let index_id = index_id.to_string();
        let path = path.use_path().to_string_lossy().to_string();
        let size = metadata.len();
        let mtime = metadata.mtime_nsec();

        let size: i64 = size.try_into()
            .expect("The file size needs to be represented in i64/u64, but the storing value was outside of the range");

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            diesel::dsl::insert_into(schema::indexed_file::table)
                .values((
                    schema::indexed_file::index_id.eq(index_id),
                    schema::indexed_file::source_file.eq(&path),
                    schema::indexed_file::source_size.eq(size),
                    schema::indexed_file::source_mtime_ns.eq(mtime),
                    schema::indexed_file::projector_version.eq(indexer),
                ))
                .on_conflict(schema::indexed_file::index_id)
                .do_update()
                .set((
                    schema::indexed_file::source_file.eq(&path),
                    schema::indexed_file::source_size.eq(size),
                    schema::indexed_file::source_mtime_ns.eq(mtime),
                    schema::indexed_file::projector_version.eq(indexer),
                ))
                .execute(&mut conn)
                .context("Failed to update the DB")?;

            Ok(())
        })
        .await
        .context("DB task panicked")?
    }
}
