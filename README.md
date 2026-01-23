# OpenCrystal

OpenCrystal is a Rust-based JRPG engine for the terminal. It targets wide compatibility
(ANSI + 256 colors) while keeping the content fully data-driven via JSON.

## Goals

- TUI-first JRPG engine with FF-style overworlds, towns, dungeons, and battles.
- Configurable systems (jobs, magic schools, ATB vs turn-based, world jumps, vehicles).
- Content driven by JSON maps, entities, events, and UI layouts.

## Project layout

- `crates/engine/`: runtime systems, loaders, validation.
- `crates/tui/`: UI configs and rendering scaffolding.
- `crates/cli/`: `cryst` command.
- `content/`: content packs (`content/demo` is tracked, includes `dialog/`).
- `SCHEMAS.md`: JSON schema drafts.
- `ARCHITECTURE.md`: architecture overview.

## Quick start

```bash
cargo run -- validate
cargo run -- play
```

Use a different content pack:

```bash
cargo run -- play --content content/demo
```

## Commands

- `cryst play [--render=auto|wide|modern] [--content path]`
- `cryst validate`
- `cryst new-project`
- `cryst build`

## Content packs

Content lives in `content/<game_name>/` with folders like `entities/`, `maps/`, `events/`, and `ui/`.
Only `content/demo` is tracked in git; other packs are ignored by default.
