# DVD Categorize — User Functionality

This document lists the top-level functionality of the DVD Categorize application from a user perspective. It is intended to be a living reference for user-facing features and behaviors, not developer implementation details.

## Table of Contents

- [Navigation](#navigation)
- [Live Search](#live-search)
- [Movie Generator](#movie-generator)
- [Insert Movies](#insert-movies)
- [Collection Statistics](#collection-statistics)
- [AI Chat Assistant](#ai-chat-assistant)
- [AI Model Management](#ai-model-management)
- [Movie Cards & Shared Actions](#movie-cards--shared-actions)

---

## Navigation

The application is a single-page web app with a top navigation bar. The root path (`/`) redirects to `/live`.

Available navigation links:

- **Live** — `/live`
- **Insert** — `/movies/new`
- **Stats** — `/stats`
- **Chat** — `/ai/chat`
- **Generator** — `/generator`
- **Models** — `/ai/models`

The Chat, Generator, and Models links are only available when the application is built with AI backend support.

---

## Live Search

The main page is the live search interface. It provides both traditional browsing and AI-assisted search across the movie catalog.

### Browse Modes

A toolbar of quick browsing actions lets users view slices of the collection without typing a query:

- **Recently Added** — shows the 50 most recently added movies ordered by the `AddedOn` field.
- **Recent Releases** — shows movies released within a recent year range (configurable by the API).
- **Random Movies** — shows 3 randomly selected movies.
- **Unknown Location** — shows movies with an empty or unset physical location.
- **Movies by Location** — choose a location from the dropdown to see all movies stored there.

### Search Modes

Users type a query and choose one of four search modes:

- **Text** — keyword and entity matching against titles, actors, directors, and genres stored in the database. Best for exact-name lookups.
- **Vector** — semantic search using AI-generated embeddings of the query and movie descriptions. Best for thematic searches (e.g., "movies about redemption"). Short queries are automatically expanded by AI unless enhancement is disabled.
- **Hybrid** — runs Text and Vector searches in parallel and merges results using Reciprocal Rank Fusion (RRF). Exact title matches are boosted. Best general-purpose mode.
- **Structured** — the AI parses natural language into structured criteria (actors, directors, genres, title keywords, description keywords) and the backend builds a safe database query. Best for multi-criteria searches like "brad pitt action adventure movies".

### Model Selection

For Vector, Hybrid, and Structured searches, a model selector allows users to choose which Ollama model drives the AI behavior. The default is used when no model is selected.

### AI Query Enhancement

For Vector and Hybrid modes, a checkbox lets users disable AI query enhancement. When enabled, short or vague queries are expanded by the AI to improve semantic results.

### Enhanced Query Display

If the AI rewrites the query, the rewritten version is shown above the results so users understand how their input was interpreted.

---

## Movie Generator

The Generator page uses AI to create movie records from a list of titles. Generated cards can be reviewed, edited, locked, and saved to the catalog.

### Title Management

- **Add titles** via an input field.
- The **pipe (`|`) separator** can be used to create multiple title chips at once (e.g., `The Matrix | Inception`).
- **Remove individual titles** by clicking the `×` on a chip.
- **Clear All** removes every title and generated result.

### Title Correction Lock

Each title chip has a lock icon:

- **Unlocked** — the AI may correct the spelling or wording of the title before generating.
- **Locked** — AI correction is disabled for that title; the title is used exactly as entered.

### AI Model Selection

A model selector allows the user to choose which Ollama model generates the movie data. The default model is used when nothing is selected.

### Generate

Clicking **Generate** streams AI-generated movie cards one at a time. The AI produces:

- Title
- Release year
- Description
- Cast list
- Director
- Genres

### Catalog Duplicate Detection

If a title already exists in the catalog, the generated card shows an **Already in catalog** badge and displays the stored movie data instead of generating new content. The card cannot be locked or saved.

### Card Actions

Each generated card has a toolbar with:

- **Save** — adds the movie to the catalog.
- **Edit** — opens an inline form to modify all fields before saving.
- **Lock** — keeps the card unchanged during re-generates.
- **Remove** — removes the card from the result grid.

### Lock & Re-generate

- **Lock a card** to preserve it when the title list changes or when Generate is clicked again.
- **Unlock a card** to allow it to be regenerated.
- A **Lock All** / **Unlock All** toolbar button above the grid toggles every card at once.
- Re-generating only produces new results for unlocked cards; locked cards are preserved.

### Streaming UX

Cards appear one by one as the AI finishes each movie. Skeleton cards are shown during generation.

---

## Insert Movies

The Insert page provides CSV-based bulk import and export of the movie catalog.

### CSV Format

The pasted CSV must contain a header row with exactly these column names:

```
Title,Year,Description,Actors,Genres,Director,AddedOn,Location
```

Column order does not matter, but it must be consistent across the file.

### Preview

Clicking **Preview** parses the CSV and displays the resulting movies in a grid without saving them to the database.

### Send to Database

After previewing, **Send to Database** imports the movies into the catalog. The backend generates vector embeddings for each movie during import.

### Export All Movies

Clicking **Export All Movies** downloads the entire catalog as a CSV file.

### Clear

**Clear** removes the preview data and resets the CSV input.

---

## Collection Statistics

The Stats page shows aggregated visual summaries of the catalog.

- **Overview** — total number of movies, directors, and actors.
- **Movies by Year** — bar chart showing the top 15 release years by movie count.
- **Genres** — pie chart showing the top 10 genre distributions.
- **Top Actors** — bar chart showing the 10 most frequent actors.

---

## AI Chat Assistant

The Chat page provides a conversational interface for asking questions about the movie collection.

### Chat Modes

- **RAG mode** (`/ai/chat/stream`) — retrieves matching movies from the catalog before streaming the response, so the AI answers from the user's collection.
- **Tool mode** (`/ai/chat`) — the LLM can invoke tools to filter by actor, genre, or director, and to fetch movie details. Returns a single non-streaming response.

### Model Selection

A model selector lets the user choose the Ollama model used for the conversation.

### Prompt Editing

Clicking **Edit Prompts** opens a panel where users can edit the system prompt for the active mode and reset it to the default.

### Chat History

- **Show History** opens a modal listing all saved chat sessions.
- Sessions display the first user query and the last updated time.
- Clicking a session loads its full history.
- **New Chat** clears the current conversation and starts a fresh session.

### Input

Users type messages in a multi-line text area. Pressing **Enter** sends the message. The AI response is rendered with Markdown formatting.

---

## AI Model Management

The Models page allows users to pull and manage Ollama models directly from the web UI.

### Pull Model

- Enter a model name (e.g., `qwen2.5:7b`, `nomic-embed-text`, `mistral`).
- Click **Pull Model** to download the model from the Ollama registry.
- The page warns that large models may take several minutes and require sufficient disk space.

### Available Models

The list of pulled models is also used by model selectors in the Live Search, Chat, and Generator pages.

---

## Movie Cards & Shared Actions

The movie grid is used throughout the app for search results, browse views, and previews.

### Movie Information

Each card displays:

- Title
- Release year
- Description
- Cast
- Director
- Genres
- Location

### Location Editing

- Clicking the location field switches to an inline editor.
- Type the new location and press **Enter** or click the save button to update it.
- Press **Escape** to cancel.
- The change is saved immediately to the database.

### Random Indicator

When viewing random movies, a banner reminds the user that the grid is showing three random selections.

### Recent Releases Year Chart

When viewing recent releases, a bar chart showing movies by year is displayed above the grid.
