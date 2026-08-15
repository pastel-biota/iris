use anyhow::Context;
use diesel::{BoolExpressionMethods as _, ExpressionMethods as _, QueryDsl as _, RunQueryDsl, dsl};

use crate::{infra::{schema::*, sqlite::SqliteConnection}, model::{EntityName, Identifier}};

#[derive(thiserror::Error, Debug)]
pub enum WhitelistRepositoryError {
    #[error("The entity is not registered")]
    NotReigstered,

    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

pub struct WhitelistRepository {
    db: SqliteConnection,
}

impl WhitelistRepository {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn photo_allowed(&self, name: &EntityName, photo_id: &Identifier) -> Result<bool, WhitelistRepositoryError> {
        let pool = self.db.pool();
        let name = name.clone();
        let photo_id = photo_id.clone();

        let exists = tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            diesel::select(dsl::exists(
                whitelist::table.filter(
                    whitelist::entity.eq(name.to_string())
                        .and(whitelist::photo_id.eq(photo_id.to_string()))
                )
            ))
                .get_result(&mut conn)
                .context("Failed on the DB query")
        })
        .await
        .context("DB task panicked")??;

        Ok(exists)
    }
}

