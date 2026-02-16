# OpenCrystal

OpenCrystal is a Rust-based JRPG engine for the terminal. It targets wide compatibility
(ANSI + 256 colors) while keeping the content fully data-driven via JSON.

## Disclaimer

This project is an original open-source game engine inspired by classic turn-based RPG
mechanics. It is not affiliated with or endorsed by any existing franchise.

## Goals

- TUI-first JRPG engine with classic renditions of overworlds, towns, dungeons, and battles.
- Configurable systems (jobs, magic schools, Readiness vs turn-based, world jumps, vehicles).
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

## Content packs

- Content packs live under `content/<pack_name>/`.
- `content/demo` is a working reference pack with dialog, events, and UI overrides.
- `--content-dir` defaults to `~/.local/share/opencrystal/content` (or `XDG_DATA_HOME/opencrystal/content`).

## CLI commands

- `cryst play [--render=auto|wide|modern] [--content path] [--content-dir path]`
- `cryst validate [--content path] [--content-dir path]`
- `cryst new-project <name> [--path path]`
- `cryst build new <kind> <id> [--content path] [--content-dir path] [--name label] [--force]`
- `cryst build upgrade [--content path] [--content-dir path] [--dry-run]`
- `cryst build new-project <name> [--path path]`

## Documentation

- `CONTENT_AUTHORING_GUIDE.md`: content pack workflow and conventions.
- `SCHEMAS.md`: authoritative JSON schemas and examples for content creation.
- `ARCHITECTURE.md`: engine design and runtime overview.
- `BATTLE_SPECS.md`: battle UI flow and interaction rules.
- `JOBS.md`: job system behavior and progression modes.
- `TODO.md`: current implementation status and backlog.
