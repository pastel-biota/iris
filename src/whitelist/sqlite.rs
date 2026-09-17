use anyhow::Context as _;
use diesel::prelude::*;

use crate::whitelist::json::WhitelistConfiguration;
use crate::{infra::schema, model::Whitelist};

use crate::infra::sqlite::SqliteConnection;

pub struct SqliteWhitelistRepository {
    db: SqliteConnection,
}

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = schema::whitelist)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct WhitelistRow {
    pub entity: String,
    pub photo_id: String,
}

impl SqliteWhitelistRepository {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn update_whitelist(&self, whitelist: WhitelistConfiguration) -> anyhow::Result<()> {
        let pool = self.db.pool();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            conn.immediate_transaction(|tx| {
                diesel::dsl::delete(schema::whitelist::table)
                    .execute(tx)
                    .context("Failed on executing the query")?;

                diesel::dsl::insert_into(schema::whitelist::table)
                    .values(
                        whitelist
                            .into_iter()
                            .flat_map(|(entity_name, photos)| {
                                let photos = match photos {
                                    Whitelist::Select { pics } => pics,
                                };

                                photos
                                    .iter()
                                    .map(|photo| {
                                        (
                                            schema::whitelist::entity.eq(entity_name.to_string()),
                                            schema::whitelist::photo_id.eq(photo.to_string()),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>(),
                    )
                    .execute(tx)
                    .context("Failed on executing the query")?;

                Ok(())
            })
        })
        .await
        .context("DB task panicked")?
    }
}

fn try_into_or_skip<T, U>(value: T) -> Option<U>
where
    T: TryInto<U>,
    T: std::fmt::Debug,
    <T as TryInto<U>>::Error: std::fmt::Display,
{
    let debug_text = format!("{:?}", value);

    match value.try_into() {
        Ok(tag) => Some(tag),
        Err(err) => {
            tracing::warn!("Invalid value, skipping this one: {}:\n{}", debug_text, err);
            None
        }
    }
}
