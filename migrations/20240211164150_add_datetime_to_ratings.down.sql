-- Add down migration script here
ALTER TABLE ratings
DROP COLUMN created_at,
DROP COLUMN updated_at;
