use std::collections::HashMap;

use anyhow::Context as _;
use diesel::prelude::*;

use crate::{
    infra::schema,
    model::{Tag, TagGroup, TagIdentifier},
    tag::types::KnownTags,
};

use crate::infra::sqlite::SqliteConnection;

pub struct SqliteTagRepository {
    db: SqliteConnection,
}

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = schema::tag_group)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TagGroupRow {
    pub group: String,
    pub display: String,
    pub annotations: Option<serde_json::Value>,
}

impl TryFrom<TagGroupRow> for TagGroup {
    type Error = anyhow::Error;

    fn try_from(value: TagGroupRow) -> anyhow::Result<Self> {
        Ok(Self {
            name: value.group.parse()?,
            display: value.display,
            annotaions: value
                .annotations
                .map(serde_json::from_value)
                .transpose()
                .context("The annotation must be object")?
                .unwrap_or_else(|| HashMap::new()),
        })
    }
}

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = schema::tag)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TagRow {
    pub name: String,
    pub group: String,
    pub display: String,
    pub annotations: Option<serde_json::Value>,
}

impl TryFrom<TagRow> for Tag {
    type Error = anyhow::Error;

    fn try_from(value: TagRow) -> anyhow::Result<Self> {
        Ok(Self {
            display: value.display,
            ident: TagIdentifier(value.group.parse()?, value.name.parse()?),
            annotaions: value
                .annotations
                .map(serde_json::from_value)
                .transpose()
                .context("The annotation must be object")?
                .unwrap_or_else(|| HashMap::new()),
        })
    }
}

impl SqliteTagRepository {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    pub async fn replace_with_known_tags_set(&self, known_tags: KnownTags) -> anyhow::Result<()> {
        let pool = self.db.pool();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            conn.immediate_transaction(|tx| {
                diesel::dsl::delete(schema::tag::table)
                    .execute(tx)
                    .context("Failed on executing the query")?;

                diesel::dsl::delete(schema::tag_group::table)
                    .execute(tx)
                    .context("Failed on executing the query")?;

                diesel::dsl::insert_into(schema::tag_group::table)
                    .values(
                        known_tags
                            .tag_groups
                            .into_iter()
                            .map(|group| {
                                (
                                    schema::tag_group::group.eq(group.name.to_string()),
                                    schema::tag_group::display.eq(group.display),
                                    schema::tag_group::annotations
                                        .eq(serde_json::to_value(group.annotaions).unwrap()),
                                )
                            })
                            .collect::<Vec<_>>(),
                    )
                    .execute(tx)
                    .context("Failed on executing the query")?;

                diesel::dsl::insert_into(schema::tag::table)
                    .values(
                        known_tags
                            .tags
                            .into_iter()
                            .map(|tag| {
                                (
                                    schema::tag::group.eq(tag.ident.0.to_string()),
                                    schema::tag::name.eq(tag.ident.1.to_string()),
                                    schema::tag::display.eq(tag.display),
                                    schema::tag::annotations
                                        .eq(serde_json::to_value(tag.annotaions).unwrap()),
                                )
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

    pub async fn get_all_known_tags(&self) -> anyhow::Result<Vec<Tag>> {
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<Tag>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let query = schema::tag::table.into_boxed();

            let tags = query
                .select(TagRow::as_select())
                .load(&mut conn)
                .context("Failed on executing the query")?
                .into_iter()
                .flat_map(try_into_or_skip)
                .collect();

            Ok(tags)
        })
        .await
        .context("DB task panicked")??;

        Ok(rows)
    }

    pub async fn get_all_known_tag_groups(&self) -> anyhow::Result<Vec<TagGroup>> {
        let pool = self.db.pool();

        let rows = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<TagGroup>> {
            let mut conn = pool.get().context("Failed to acquire DB connection")?;

            let query = schema::tag_group::table.into_boxed();

            let tag_groups = query
                .select(TagGroupRow::as_select())
                .load(&mut conn)
                .context("Failed on executing the query")?
                .into_iter()
                .flat_map(try_into_or_skip)
                .collect();

            Ok(tag_groups)
        })
        .await
        .context("DB task panicked")??;

        Ok(rows)
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
