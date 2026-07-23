CREATE TABLE whitelist (
  entity text NOT NULL,
  photo_id text NOT NULL,

  FOREIGN KEY(photo_id) REFERENCES photos(id)
);

CREATE INDEX whitelist_entity_photoid ON whitelist (entity, photo_id);
