use diesel::{prelude::*, sqlite::Sqlite};

use crate::{
    infra::schema::{photos, whitelist},
    model::{Identifier, PhotoVisibility},
};

type PhotoQuery<'a> = photos::BoxedQuery<'a, Sqlite>;

pub(super) fn select_visible_photos<'a>(visibility: &PhotoVisibility) -> PhotoQuery<'a> {
    #[allow(clippy::disallowed_methods)]
    let query = photos::table.into_boxed();
    let query = query.order_by((photos::shot_time_unix.desc(), photos::id.desc()));

    match visibility {
        PhotoVisibility::All => query,
        PhotoVisibility::WhitelistedFor(entity_name) => query.filter(diesel::dsl::exists(
            whitelist::table.filter(
                whitelist::photo_id
                    .eq(photos::id)
                    .and(whitelist::entity.eq(entity_name.to_string())),
            ),
        )),
    }
}

pub(super) fn select_after_cursor<'a, 'cur: 'a>(
    query: PhotoQuery<'a>,
    cursor: &'cur (i64, Identifier),
) -> PhotoQuery<'a> {
    let (begin_shot_time_unix, begin_id) = cursor;
    let begin_id = begin_id.to_string();

    query.filter(
        photos::shot_time_unix
            .lt(begin_shot_time_unix)
            .or(photos::shot_time_unix
                .eq(begin_shot_time_unix)
                .and(photos::id.lt(begin_id))),
    )
}
