-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS movie_actor_order_idx;
ALTER TABLE movie_actor DROP COLUMN actor_order;
