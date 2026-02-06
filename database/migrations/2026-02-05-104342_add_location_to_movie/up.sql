-- Add location column for physical storage location (e.g., "Action 1", "Comedy 1")
ALTER TABLE movie ADD COLUMN location VARCHAR(100) NOT NULL DEFAULT 'Unknown';
