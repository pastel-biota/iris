diesel::table! {
    photos (id) {
        id -> Text,
        federated_by -> Nullable<Text>,
        shot_time_unix -> BigInt,
        original_sha256 -> Text,
        meta_json -> Text,
    }
}

diesel::table! {
    whitelist (rowid) {
        rowid -> BigInt,
        entity -> Text,
        photo_id -> Text,
    }
}

diesel::joinable!(whitelist -> photos (photo_id));
diesel::allow_tables_to_appear_in_same_query!(photos, whitelist);
