-- Enable pg_trgm extension for trigram-based text search
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Create GIN indexes for ILIKE pattern matching on searchable columns
CREATE INDEX idx_movie_name_trgm ON movie USING gin (name gin_trgm_ops);
CREATE INDEX idx_actor_name_trgm ON actor USING gin (name gin_trgm_ops);
CREATE INDEX idx_director_name_trgm ON director USING gin (name gin_trgm_ops);
CREATE INDEX idx_genre_name_trgm ON movie_genre USING gin (genre gin_trgm_ops);

-- Add statistics for better query planning
ALTER TABLE movie ALTER COLUMN name SET STATISTICS 1000;
ALTER TABLE actor ALTER COLUMN name SET STATISTICS 1000;
ALTER TABLE director ALTER COLUMN name SET STATISTICS 1000;
ALTER TABLE movie_genre ALTER COLUMN genre SET STATISTICS 1000;
