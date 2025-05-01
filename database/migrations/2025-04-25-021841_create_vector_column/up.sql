-- Your SQL goes here
ALTER TABLE movie
ADD COLUMN embedding VECTOR (768);

CREATE INDEX movies_embedding_idx ON movie USING hnsw (embedding vector_cosine_ops)
WITH
    (m = 16, ef_construction = 64);
