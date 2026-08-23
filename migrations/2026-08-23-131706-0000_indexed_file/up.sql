CREATE TABLE `indexed_file`(
	`index_id` TEXT NOT NULL PRIMARY KEY,
	`source_file` TEXT NOT NULL,
	`source_mtime_ns` BIGINT NOT NULL,
	`source_size` BIGINT NOT NULL,
	`projector_version` TEXT NOT NULL
);

CREATE INDEX indexed_file_source_file ON indexed_file (source_file);

