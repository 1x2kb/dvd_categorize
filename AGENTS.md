# Project: dvd_categorize

Full-stack DVD collection app. Rust workspace, 8+ crates. Axum API, Dioxus/WASM
frontend, PostgreSQL (Diesel) XOR MongoDB backend, Ollama for local LLM features,
Qdrant for vector search (mongodb backend only).

Crates: dvd_catalog_api, dvd_categorizer_web, ai_chat, database, models,
csv_utils, categorizer_utilities, plus actor_reorder, dashboards, dioxus_grapher.

## Backend features

`postgres` and `mongodb` are MUTUALLY EXCLUSIVE — a compile-time check enforces it.
Never use --all-features. Features are per-crate and use the `crate/feature` form.

A backend feature is mandatory on every cargo command. See build.md for the
exact invocations.

## Mirrored implementations

Backend-specific code is mirrored: e.g. `database/src/mongodb.rs` has a postgres
counterpart. When you change one, check whether the other needs the same change.

## Paths

Paths are relative to the repo root. Never prefix with `/` or `~`.
  Correct: src/components/movie_grid.rs
  Wrong:   /~/Workspace/project/src/components/movie_grid.rs

## Tooling that already exists

- `./clippy-mongodb.sh` / `./clippy-postgres.sh` — use these, don't hand-roll clippy
- `make clippy-mongodb` / `make clippy-postgres`
- `make e2e*` — Playwright suite (needs `make e2e-install` first)
- `rustfmt.toml` is configured — run `cargo fmt` before declaring done

## Naming gotcha

`prompts/` is the application's own crate (Ollama prompt templates, conversions
module). It is NOT where agent prompts live. Agent definitions are in
`.opencode/agents/`.