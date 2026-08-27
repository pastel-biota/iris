CREATE TABLE `tag`(
	`name` TEXT NOT NULL PRIMARY KEY,
	`group` TEXT NOT NULL,
	`display` TEXT NOT NULL
  -- FOREIGN KEY claude is omitted as the `tag` and `tag_group` does not
  -- share the lifecycle (tag stays valid even if the tag_group gets removed)
	-- FOREIGN KEY (`group`) REFERENCES `tag_group`(`group`)
);

CREATE TABLE `tag_group`(
	`group` TEXT NOT NULL PRIMARY KEY,
	`display` TEXT NOT NULL
);

CREATE TABLE `photo_tag`(
	`photo_id` TEXT NOT NULL,
	`tag_identifier` TEXT NOT NULL,

	`tag_group` TEXT NOT NULL GENERATED ALWAYS
    AS (substr(`tag_identifier`, 1, instr(`tag_identifier`, '/') - 1))
    VIRTUAL,

	`tag_name` TEXT NOT NULL GENERATED ALWAYS
    AS (substr(`tag_identifier`, instr(`tag_identifier`, '/') + 1))
    VIRTUAL,

	PRIMARY KEY(`photo_id`, `tag_identifier`),

	FOREIGN KEY (`photo_id`) REFERENCES `photos`(`id`)
) WITHOUT ROWID;

CREATE INDEX photo_tag_by_identifier
ON photo_tag(tag_identifier, photo_id);

CREATE INDEX photo_tag_by_group
ON photo_tag(tag_group, photo_id);

