# CSV Utils Module

CSV parsing and serialization utilities for movie data import/export.

## Overview

This crate provides utilities for parsing CSV data into movie objects and serializing movies back to CSV format. It's used by the web UI's Insert page for importing movies and the export endpoint for downloading the collection.

## Features

### CSV Parsing

Parses CSV data with the following columns:
- Movie name
- Director
- Description
- Actors (comma-separated)
- Genres (comma-separated)
- Location (optional)

```rust
use csv_utils::parse_csv;

let csv_data = b"Movie Name,Director,Description,Actors,Genres,Location\n...";
let movies = parse_csv(csv_data)?;
```

### CSV Export

Serializes a collection of movies to CSV format:

```rust
use csv_utils::movies_to_csv;

let movies = vec![/* ... */];
let csv_string = movies_to_csv(&movies)?;
```

## CSV Format

### Expected Input Format

```csv
Movie Name,Director,Description,Actors,Genres,Location
The Matrix,The Wachowskis,A computer hacker learns about reality,"Keanu Reeves,Laurence Fishburne","Sci-Fi,Action",Shelf A
Inception,Christopher Nolan,Dream heist thriller,"Leonardo DiCaprio,Tom Hardy","Sci-Fi,Thriller",Shelf B
```

### Notes

- **Headers**: First row must contain column headers (case-insensitive)
- **Actors**: Multiple actors separated by commas within quotes
- **Genres**: Multiple genres separated by commas within quotes
- **Location**: Optional field for physical storage location
- **Quotes**: Use quotes around fields containing commas
- **Empty Fields**: Director and description can be empty

## Usage in Application

### Import Flow

1. User pastes CSV data in the web UI Insert page
2. Frontend sends data to `/csv/preview` endpoint
3. `parse_csv()` validates and parses the data
4. Preview is shown to user
5. User confirms, frontend sends to `/csv/parse` endpoint
6. Movies are inserted into database with embeddings generated

### Export Flow

1. User clicks export button
2. Frontend requests `/csv/export` endpoint
3. `movies_to_csv()` serializes all movies
4. Browser downloads the CSV file

## Error Handling

The functions return `Result<T, String>` with descriptive error messages for:
- Invalid CSV format
- Missing required columns
- Malformed data

## Dependencies

- `csv`: CSV parsing and writing
- `models`: Movie domain types
- `serde`: Serialization support

## Feature Flags

- **postgres**: Enables PostgreSQL-specific types in models
- **vector-similarity**: Enables vector similarity features in models

Both features are enabled by default.
