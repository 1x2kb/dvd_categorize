-- Your SQL goes here
ALTER TABLE movies
ALTER COLUMN embedding TYPE vector (768);

CREATE INDEX movies_embedding_idx ON movies USING hnsw (embedding vector_cosine_ops)
WITH
    (m = 16, ef_construction = 64);
