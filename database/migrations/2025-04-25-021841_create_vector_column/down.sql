-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS movies_embedding_idx;

ALTER TABLE movie
DROP COLUMN embedding;
