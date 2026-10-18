## `AGENTS.md`

Project root. This is what teaches the primary how to consume Strand's output.

```markdown
# Agent conventions

## Paths
Tool paths are relative to the repo root. Never prefix with `/` or `~`.
  Correct: src/components/movie_grid.rs
  Wrong:   /~/Workspace/project/src/components/movie_grid.rs

## Rust work
- Delegate any non-trivial Rust change to the `rust` subagent via the Task tool.
- The `rust` subagent is read-only and returns `EDIT` blocks. Apply each one
  with the `edit` tool using the `file`, `old`, and `new` values VERBATIM.
  Do not paraphrase `old` — it must match the file byte-for-byte.
- If an edit fails to apply, re-read the file and retry once, then fix it yourself.

## Verification
- Run `cargo check --message-format=short` after every change.
- Run `cargo clippy` before declaring a task complete.
- Run `cargo test` if the change touched logic, not just types.
