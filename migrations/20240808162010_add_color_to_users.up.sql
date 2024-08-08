-- Add up migration script here
ALTER TABLE users
ADD COLUMN color VARCHAR(7) NOT NULL;
