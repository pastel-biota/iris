-- Your SQL goes here
ALTER TABLE `tag` ADD COLUMN `annotations` JSONB;
ALTER TABLE `tag_group` ADD COLUMN `annotations` JSONB;
