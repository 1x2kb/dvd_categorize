# Project: dvd_categorize

Cargo workspace. Crates: database/, dvd_catalog_api/, dvd_categorizer_web/,
categorizer_utilities/, csv_utils/, models/, ai_tools/, actor_reorder/.

## Paths
Paths are relative to the repo root. Never prefix with `/` or `~`.
  Correct: src/components/movie_grid.rs
  Wrong:   /~/Workspace/project/src/components/movie_grid.rs

## Verification
- `cargo check --message-format=short` after every change
- `cargo clippy` before declaring a task complete
- `cargo test` if the change touched logic, not just types