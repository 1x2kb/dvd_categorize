-- Your SQL goes here
-- Add actor_order column to track billing order (1 = top-billed, 2 = second, etc.)
ALTER TABLE movie_actor ADD COLUMN actor_order INTEGER NOT NULL DEFAULT 0;

-- Set initial order based on current insertion order (id)
-- This preserves existing order as best we can
WITH ranked_actors AS (
    SELECT 
        id,
        ROW_NUMBER() OVER (PARTITION BY movie_id ORDER BY id) as row_num
    FROM movie_actor
)
UPDATE movie_actor
SET actor_order = ranked_actors.row_num
FROM ranked_actors
WHERE movie_actor.id = ranked_actors.id;

-- Remove the default now that data is populated
ALTER TABLE movie_actor ALTER COLUMN actor_order DROP DEFAULT;

-- Create index for efficient ordering queries
CREATE INDEX movie_actor_order_idx ON movie_actor (movie_id, actor_order);
