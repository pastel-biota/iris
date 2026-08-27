-- Your SQL goes here

CREATE TABLE photos_v2 (
    id TEXT PRIMARY KEY NOT NULL,
    federated_by TEXT,
    cached_meta_json TEXT NOT NULL CHECK (json_valid(cached_meta_json)),
    shot_time_unix INTEGER NOT NULL,

    original_sha256 TEXT NOT NULL
        GENERATED ALWAYS AS (
            json_extract(cached_meta_json, '$.original_sha256')
        ) STORED
);

CREATE INDEX photos_v2_shot_time_unix
    ON photos_v2 (shot_time_unix);

CREATE INDEX photos_v2_original_sha256
    ON photos_v2 (original_sha256);

ALTER TABLE photos RENAME TO _to_be_removed_photos;
ALTER TABLE photos_v2 RENAME TO photos;

