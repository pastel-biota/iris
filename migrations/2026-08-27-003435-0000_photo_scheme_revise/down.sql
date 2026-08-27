-- This file should undo anything in `up.sql`

DROP TABLE photos;
ALTER TABLE _to_be_removed_photos RENAME TO photos;
