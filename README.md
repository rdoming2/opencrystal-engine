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

## Capabilities

- **Rendering:** ANSI + 256 color output with wide/modern render modes, palette-based colors, and battle breakpoints for glyph vs ASCII art.
- **Worlds & maps:** multi-world travel, overworld/town/dungeon maps, encounter zones, transitions, save points, vehicles, and overworld map menus with auto-downsampled views.
- **Events & dialog:** event scripts with dialog/narration, flags, grants, warps, battles, NPC show/hide/move/sprite, shops, rests, and stat updates; triggers on map enter/step with zone or coordinate targeting; dialog trees with actions and gated choices.
- **Battle system:** turn-based and Readiness modes, pause, command catalog with job extensions, target rules, front/back rows, boss scaling, elements/traits/status effects, and victory/defeat rewards.
- **Jobs & progression:** character/job/job_points progression modes, optional secondary jobs, job command lists, spell/ability unlocks by level/item/equip/jp, magic equip slots, and tier charge tables.
- **Party & inventory:** create/preset/preset_rename party modes, roster + reserve, equipment slots, inventory stacks, items usable in field/battle, and campfire cooking.
- **UI & UX:** two-pane main menu, journal/quest tracking, gameplay stats panels, settings visibility/locking, localized UI strings, and configurable title/menu/battle layouts.
- **Data & tooling:** JSON schemas with validation, content pack metadata, `cryst build` stubs/upgrades, and JSON save files with autosave slot support.

See `SCHEMAS.md` and `CONTENT_AUTHORING_GUIDE.md` for schema-level details and authoring workflows.

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
