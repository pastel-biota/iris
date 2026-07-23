CREATE TABLE photos (
  id text PRIMARY KEY NOT NULL,
  federated_by text,
  shot_time_unix integer NOT NULL,  -- Sqlite does not have datetime
  original_sha256 text NOT NULL,
  meta_json text NOT NULL
);

CREATE INDEX photos_shot_time_unix ON photos (shot_time_unix);
CREATE INDEX photos_original_sha256 ON photos (original_sha256);
