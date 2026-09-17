-- This file should undo anything in `up.sql`
ALTER TABLE `tag` DROP COLUMN `annotations`;
ALTER TABLE `tag_group` DROP COLUMN `annotations`;
