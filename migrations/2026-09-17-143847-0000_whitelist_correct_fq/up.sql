CREATE TABLE whitelist_v2 (
  entity text NOT NULL,
  photo_id text NOT NULL
);

CREATE INDEX whitelist_v2_entity
    ON whitelist_v2 (entity);

CREATE INDEX whitelist_v2_entity_photo_id
    ON whitelist_v2 (entity, photo_id);

INSERT INTO whitelist_v2 (entity, photo_id) SELECT entity, photo_id FROM whitelist;

ALTER TABLE whitelist RENAME TO _to_be_removed_whitelist;
ALTER TABLE whitelist_v2 RENAME TO whitelist;

DROP TABLE _to_be_removed_whitelist;

