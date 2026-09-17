diesel::table! {
    photos (id) {
        id -> Text,
        federated_by -> Nullable<Text>,
        shot_time_unix -> BigInt,
        original_sha256 -> Text,
        cached_meta_json -> Text,
    }
}

diesel::table! {
    whitelist (rowid) {
        rowid -> BigInt,
        entity -> Text,
        photo_id -> Text,
    }
}

diesel::table! {
    indexed_file (index_id) {
        index_id -> Text,
        source_file -> Text,
        source_mtime_ns -> BigInt,
        source_size -> BigInt,
        projector_version -> Text,
    }
}

diesel::joinable!(whitelist -> photos (photo_id));
diesel::allow_tables_to_appear_in_same_query!(photos, whitelist);

diesel::table! {
    tag_group (group) {
        group -> Text,
        display -> Text,
        annotations -> Nullable<Jsonb>,
    }
}

diesel::table! {
    tag (name) {
        name -> Text,
        group -> Text,
        display -> Text,
        annotations -> Nullable<Jsonb>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(tag_group, tag);
diesel::joinable!(tag -> tag_group (group));

diesel::table! {
    photo_tag (photo_id, tag_identifier) {
        photo_id -> Text,
        tag_identifier -> Text,
        tag_group -> Text,
        tag_name -> Text,
    }
}
diesel::allow_tables_to_appear_in_same_query!(photos, photo_tag);
diesel::allow_tables_to_appear_in_same_query!(photo_tag, tag);
diesel::allow_tables_to_appear_in_same_query!(photo_tag, tag_group);
diesel::joinable!(photo_tag -> photos (photo_id));
diesel::joinable!(photo_tag -> tag_group (tag_group));
diesel::joinable!(photo_tag -> tag (tag_name));
