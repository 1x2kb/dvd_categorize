-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS movie_release_year_idx;
DROP INDEX IF EXISTS movie_name_year_unique_idx;

-- Restore movie names with years before dropping the column
UPDATE movie
SET name = name || ' (' || release_year || ')'
WHERE release_year IS NOT NULL;

ALTER TABLE movie DROP COLUMN release_year;

-- Restore the original unique constraint on name only
CREATE UNIQUE INDEX movie_name_index ON movie (name);
