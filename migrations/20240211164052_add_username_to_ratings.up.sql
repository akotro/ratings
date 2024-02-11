-- Add up migration script here
ALTER TABLE ratings
ADD COLUMN username VARCHAR(255) NOT NULL;
