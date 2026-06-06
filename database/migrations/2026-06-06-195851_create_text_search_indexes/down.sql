-- Drop GIN indexes
DROP INDEX IF EXISTS idx_movie_name_trgm;
DROP INDEX IF EXISTS idx_actor_name_trgm;
DROP INDEX IF EXISTS idx_director_name_trgm;
DROP INDEX IF EXISTS idx_genre_name_trgm;

-- Reset statistics to default (100)
ALTER TABLE movie ALTER COLUMN name SET STATISTICS 100;
ALTER TABLE actor ALTER COLUMN name SET STATISTICS 100;
ALTER TABLE director ALTER COLUMN name SET STATISTICS 100;
ALTER TABLE movie_genre ALTER COLUMN genre SET STATISTICS 100;
