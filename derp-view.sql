\set ON_ERROR_STOP true

DROP INDEX IF EXISTS flutterbitch_tags_tmp;
DROP MATERIALIZED VIEW IF EXISTS flutterbitch_tmp;

CREATE MATERIALIZED VIEW flutterbitch_tmp AS
SELECT
    images.id AS id,
    MIN(images.version_path) AS path,
    MIN(images.image_name) AS name,
    MIN(images.image_format) AS filext,
    to_char(MIN(images.created_at), 'YYYY-MM-DD"T"HH24:MI:SSOF') AS time,
    array_agg(tags.name) AS tags
FROM
    images,
    image_taggings,
    tags
WHERE
    images.image_format IN ('png', 'jpg', 'jpeg')
    AND images.score >= 50
    AND image_taggings.image_id = images.id
    AND image_taggings.tag_id = tags.id
GROUP BY
    images.id
HAVING
    NOT (array_agg(tags.name) && '{blacklist,goes,here}'::text[])
;

CREATE INDEX flutterbitch_tags_tmp ON flutterbitch_tmp USING GIN (tags);

DROP INDEX IF EXISTS flutterbitch_tags;
DROP MATERIALIZED VIEW IF EXISTS flutterbitch;

ALTER MATERIALIZED VIEW flutterbitch_tmp RENAME TO flutterbitch;
ALTER INDEX flutterbitch_tags_tmp RENAME TO flutterbitch_tags;
