-- Add down migration script here
ALTER TABLE ratings
DROP FOREIGN KEY fk_ratings_group_id,
DROP COLUMN group_id;

DROP TABLE group_memberships;

DROP TABLE groups;
