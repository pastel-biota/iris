ALTER TABLE whitelist RENAME TO whitelist_v2;

CREATE TABLE whitelist_v1 (
  entity text NOT NULL,
  photo_id text NOT NULL,

  FOREIGN KEY(photo_id) REFERENCES _to_be_removed_photos(id)
);

CREATE INDEX whitelist_v1_entity_photo_id
    ON whitelist_v1 (entity, photo_id);

INSERT INTO whitelist_v1 (entity, photo_id) SELECT entity, photo_id FROM whitelist_v2;

ALTER TABLE whitelist_v2 RENAME TO _to_be_removed_whitelist;
ALTER TABLE whitelist_v1 RENAME TO whitelist;

DROP TABLE _to_be_removed_whitelist;

