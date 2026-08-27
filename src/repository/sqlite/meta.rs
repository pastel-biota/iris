use diesel::prelude::*;

use crate::{infra::schema, model::PhotoMeta};

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = schema::photos)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PhotoTableRow {
    pub id: String,
    pub cached_meta_json: String,
}

pub struct ParsedPhotoTableRow {
    meta: PhotoMeta,
}

impl TryFrom<PhotoTableRow> for ParsedPhotoTableRow {
    type Error = anyhow::Error;

    fn try_from(row: PhotoTableRow) -> Result<Self, Self::Error> {
        let meta = match serde_json::from_str::<PhotoMeta>(&row.cached_meta_json) {
            Ok(meta) => meta,
            Err(error) => anyhow::bail!(
                "Reference JSON in DB has corruption: for ID {}: {}",
                row.id,
                error
            ),
        };

        Ok(ParsedPhotoTableRow { meta })
    }
}

pub fn rows_into_reference(rows: Vec<PhotoTableRow>) -> Vec<PhotoMeta> {
    rows.into_iter()
        .flat_map(|row| match ParsedPhotoTableRow::try_from(row) {
            Ok(row) => Some(row.meta),
            Err(err) => {
                tracing::warn!("{err}");
                None
            }
        })
        .collect()
}
