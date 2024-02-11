-- Add down migration script here
ALTER TABLE ratings
DROP COLUMN username;
