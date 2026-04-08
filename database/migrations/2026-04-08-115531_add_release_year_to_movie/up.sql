-- Your SQL goes here
ALTER TABLE movie ADD COLUMN release_year INTEGER;

-- Drop the unique constraint on name temporarily
DROP INDEX IF EXISTS movie_name_index;

-- Extract year from movie names in format "Name (Year)" and update both columns
UPDATE movie
SET 
    release_year = CAST(
        SUBSTRING(
            name FROM '\((\d{4})\)$'
        ) AS INTEGER
    ),
    name = TRIM(
        REGEXP_REPLACE(name, '\s*\(\d{4}\)$', '')
    )
WHERE name ~ '\(\d{4}\)$';

-- Recreate unique constraint on (name, release_year) combination
CREATE UNIQUE INDEX movie_name_year_unique_idx ON movie (name, release_year);

CREATE INDEX movie_release_year_idx ON movie (release_year);
