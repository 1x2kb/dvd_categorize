-- Add added_on column with default to current timestamp
ALTER TABLE movie ADD COLUMN added_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP;
