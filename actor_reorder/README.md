# Actor Reorder

A binary utility that uses Ollama AI to reorder movie actors from top-billed to least-billed based on the AI's knowledge of each film.

## Purpose

Movie actor lists are often in arbitrary order. This tool leverages Ollama's knowledge to reorder actors according to typical billing practices (lead actors first, supporting actors last).

## Requirements

- Ollama running locally or accessible via network
- A compatible model (default: `phi3.5`)

## Usage

### Basic Usage

```bash
cargo run --bin actor_reorder -- <input_csv> [output_csv]
```

### Examples

```bash
# Reorder actors in movies.csv, output to movies_reordered.csv (auto-generated name)
cargo run --bin actor_reorder -- "movies (2).csv"

# Specify custom output file
cargo run --bin actor_reorder -- "movies (2).csv" "movies_corrected.csv"
```

### Environment Variables

- `OLLAMA_HOST` - Ollama server host (default: `localhost`)
- `OLLAMA_PORT` - Ollama server port (default: `11434`)
- `OLLAMA_MODEL` - Model to use for reordering (default: `phi3.5`)
- `RUST_LOG` - Set logging level (e.g., `info`, `debug`)

### Example with Environment Variables

```bash
OLLAMA_HOST=192.168.1.100 OLLAMA_PORT=11434 RUST_LOG=info \
  cargo run --bin actor_reorder -- "movies (2).csv"
```

## How It Works

1. Reads the input CSV file containing movie data
2. For each movie with actors:
   - Sends the movie title, year, description, and actor list to Ollama
   - Asks the AI to reorder actors by billing importance
   - Updates the movie record with the reordered actors
3. Writes the updated movie data to the output CSV file

## Output

The tool provides detailed logging showing:
- Progress through the movie list
- Original actor order
- AI-reordered actor order
- Any errors encountered

Movies without actors are skipped. If the AI fails to reorder actors for a specific movie, the original order is preserved.

## Notes

- Ollama does not have internet access, so it relies on its training data knowledge
- The AI makes its best guess based on the movie title, year, and description
- Results may vary depending on the model used and the obscurity of the film
- The original CSV file is never modified - a new file is always created
