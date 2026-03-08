# Agent Notes

- Keep `ARCHITECTURE.md`, `SCHEMAS.md`, `README.md`, and `TODO.md` in sync with feature changes.
- Run `cargo check` after code changes unless explicitly told otherwise.
- Menu subviews should keep the main menu on the left and render details on the right.
- Prefer extending existing schema fields rather than adding new files.
- For UI tweaks, keep the TUI styling consistent (yellow for selection, gray for disabled, cyan for equipped/accents).
- Write context-appropriate tests: default to deterministic unit tests with in-memory/local fixtures, avoid relying on unversioned `content/` data in default test runs, and mark environment-dependent tests with `#[ignore]`.
