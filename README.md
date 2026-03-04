# OpenCrystal

OpenCrystal is a Rust-based JRPG engine for the terminal. It targets wide compatibility
(ANSI + 256 colors) while keeping the content fully data-driven via JSON.

```text
    .   .   /#\     .
  *        /*##\  .    *
'      .  /**###\    .  '
    <    /***####\      >
  .   *  \***####/  *   .
*       . \**###/   .    *
    .      \*##/  *      .
  '      *  \#/  .    '
     OpenCrystal Engine
```

## Disclaimer

This project is an original open-source game engine inspired by classic turn-based RPG
mechanics. It is not affiliated with or endorsed by any existing franchise.

## Goals

- TUI-first JRPG engine with classic renditions of overworlds, towns, dungeons, and battles.
- Configurable systems (jobs, magic schools, Readiness vs turn-based, world jumps, vehicles).
- Content driven by JSON maps, entities, events, and UI layouts.

## Quick start (binary)

If you have the `cryst` binary (for example from a release zip), you can run it directly:

```bash
./cryst play
```

Use a specific content pack:

```bash
./cryst play --content content/demo
```

Validate a pack:

```bash
./cryst validate --content content/demo
```

## Quick start (source)

```bash
cargo run -- validate
cargo run -- play
```

Use a different content pack:

```bash
cargo run -- play --content content/demo
```

## Capabilities

OpenCrystal aims to cover the full loop of a classic JRPG, but keeps the content authoring
data-driven and inspectable. Highlights include:

| Area | Highlights |
| --- | --- |
| Rendering | ANSI + 256 colors, palette-based styling, wide/modern modes, glyph or ASCII battle art based on terminal size. |
| Worlds & travel | Multi-world maps, overworld/town/dungeon layers, encounters, transitions, fast travel, vehicles, and downsampled overworld map views. |
| Events & dialog | Scripted events with flags, warps, battles, shops, rests, stat updates, and dialog trees with actions and gated choices. |
| Battle | Turn-based and Readiness modes, pause, command catalogs, targeting rules, rows, traits/status effects, boss scaling (by level), difficulty scaling (global), and configurable formulas. |
| Jobs & growth | Character/job/job_points/activity progression, post-battle stat growth, JP earn/spend modes, optional secondary jobs, spell/ability unlock modes (including equipment-granted), tier charges, and magic equip slots. |
| Party & inventory | Create/preset/preset_rename party flows, roster + reserve, equipment slots, inventory stacks, field/battle item use, cooking, multi-currency support, and persistent merchant pools/stock. |
| UI & UX | Two-pane main menu, journal and gameplay stats panels, settings visibility/locking, localized strings, title and battle layout configs. |
| Tooling | Validation, content stubs, a TUI map editor, and docs output for automation workflows. |

See `SCHEMAS.md`, `ARCHITECTURE.md`, `JOBS.md` and `CONTENT_AUTHORING_GUIDE.md` for deeper references.

## Project layout

- `crates/engine/`: runtime systems, loaders, validation.
- `crates/tui/`: UI configs and rendering scaffolding.
- `crates/cli/`: `cryst` command.
- `SCHEMAS.md`: JSON schema drafts.
- `ARCHITECTURE.md`: architecture overview.

## Content packs

- Content packs live under `content/<pack_name>/`.
- `cryst play` opens a content chooser when `--content` is omitted, listing packs under `--content-dir`.
- `--content-dir` defaults to `~/.local/share/opencrystal/content` (or `XDG_DATA_HOME/opencrystal/content`, or `%LOCALAPPDATA%\opencrystal\content` on Windows).

## CLI commands

- `cryst play [--render=auto|wide|modern] [--content path] [--content-dir path]`
- `cryst validate [--content path] [--content-dir path]`
- `cryst new-project <name> [--path path]`
- `cryst build new <kind> <id> [--content path] [--content-dir path] [--name label] [--force]`
- `cryst build map <id> [--content path]`
- `cryst build upgrade [--content path] [--content-dir path] [--dry-run]`
- `cryst build strings [--content path] [--content-dir path] [--force]`
- `cryst build new-project <name> [--path path]`
- `cryst build docs [-s|--schemas] [-a|--architecture] [-c|--content-authoring] [-j|--jobs]`
- `cryst completion <bash|zsh>`

Enable shell completion:

```bash
# Bash (current shell)
source <(cryst completion bash)

# Zsh (current shell)
source <(cryst completion zsh)
```

The completion scripts include command/subcommand suggestions and path completion for
`--content`, `--content-dir`, and `--path`.

`cryst build new` kinds:

- `spell`, `ability`, `item`, `equipment`, `enemy`, `vehicle`, `shop`, `npc`, `encounter`, `job`

Examples:

```bash
cryst build new spell cure --content content/demo
cryst build new npc innkeeper --content content/demo --name "Coral Innkeeper"
```

## Build tools (CLI)

- `cryst build new`: scaffolds JSON stubs for entities like spells, items, and NPCs.
- `cryst build upgrade`: fills in missing defaults and reports extra fields.
- `cryst build strings`: writes a `ui/strings.json` localization stub.
- `cryst build map`: launches the TUI map editor for `maps/<id>.json`.
  - Tile paint, rectangular visual selection, yank/paste, undo/redo.
  - Move mode: press `m` to pick an object at the cursor; move it with arrows/HJKL; press `m` again to place.

## Documentation

- `CONTENT_AUTHORING_GUIDE.md`: content pack workflow and conventions.
- `SCHEMAS.md`: authoritative JSON schemas and examples for content creation.
- `ARCHITECTURE.md`: engine design and runtime overview.
- `BATTLE_SPECS.md`: battle UI flow and interaction rules.
- `JOBS.md`: job system behavior and progression modes.
- `TODO.md`: current implementation status and backlog.

Use `cryst build docs` to print documentation to stdout for LLM workflows.
