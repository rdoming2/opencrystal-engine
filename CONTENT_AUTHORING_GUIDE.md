# Content Authoring Guide

This guide describes how to build OpenCrystal content packs that validate cleanly and
follow the project's data-driven conventions. It focuses on JSON authoring, event
structure, and practical workflow tips for building playable content.

## Related docs

- `README.md` for project overview and CLI basics.
- `SCHEMAS.md` for authoritative JSON schema details and examples.

## Scope

Use this guide when you are writing or maintaining content under `content/<pack_name>/`.

## Content pack layout

Content packs live under `content/<pack_name>/` and typically include:

- `rules.json`: global settings, system toggles, and game metadata.
- `worlds.json`: world list and travel links.
- `stats.json`: base and derived stat definitions.
- `input.json`: optional key binding overrides.
- `party.json`: optional preset party roster for preset or preset_rename modes.
- `effects.json`: status, trait, and elemental definitions.
- `entities/`: jobs, items, equipment, spells, abilities, enemies, vehicles, shops, encounters, npcs.
- `maps/`: overworlds, towns, dungeons, and their event triggers.
- `events/`: event scripts and cutscenes.
- `dialog/`: dialog trees for NPC conversations.
- `ui/`: menu, title, battle, dialog, and gameplay stats layouts.

## Authoring workflow

1. Start from a known-good base (copy `content/demo` or an existing pack).
   - Or scaffold a minimal pack with `cryst build new-project <name>`.
2. Define game metadata in `rules.json` (title, description, author).
3. Build your world graph in `worlds.json` and decide starting location.
4. Create map layouts and encounter zones in `maps/*.json`.
5. Add entities and encounters under `entities/` and link them by ID.
6. Script story flow in `events/*.json` and `dialog/*.json`.
7. Configure UI layouts in `ui/` only when you need changes from defaults.
8. Run `cryst validate` frequently and fix all schema warnings/errors.
9. Playtest with `cryst play --content <pack_path>` and iterate.
   - Use `--content-dir <path>` to point at the directory that contains your packs (defaults to `~/.local/share/opencrystal/content`, `XDG_DATA_HOME/opencrystal/content`, or `%LOCALAPPDATA%\opencrystal\content` on Windows).

## Builder commands

- `cryst build new <kind> <id> --content <pack_path>` adds a schema stub.
  - Kinds: `spell`, `ability`, `item`, `equipment`, `enemy`, `vehicle`, `shop`, `npc`, `encounter`, `job`.
- `cryst build map <id> --content <pack_path>` opens the TUI map editor for `maps/<id>.json`.
- `cryst build upgrade --content <pack_path>` fills in missing defaults and reports extra fields.
- `cryst build docs` prints reference documentation to stdout.

Examples:

```bash
cryst build new item potion --content content/demo
cryst build new encounter grasslands_skirmish --content content/demo
```

## Core conventions

- IDs are lowercase snake_case across all files.
- Reference other entities by ID, not by filename.
- Prefer extending existing schema fields instead of inventing new files or formats.
- Use flags for progression and gating (`quest.*`, `map.*`, `system.*`, `vehicle.*`, `dungeon.*`).
- Palette values use terminal color names (e.g., `bright_yellow`).
- Keep dialog text concise; it will wrap automatically.

## Maps and traversal

- Use `legend` entries to map tile glyphs to tile definitions.
- `encounters` describe zones by rectangle and link to encounter tables by ID.
- `allow_save` and `save_points` define where saving is permitted.
- `transitions` should include `target_map` and `target_pos` for deterministic travel.
- `npcs.requires_flags` hides NPCs (rendering/collision/interaction) until all flags are set.

## Events and dialog

- Use `events/*.json` for linear scripts and cutscenes.
- Use `dialog/*.json` for branching NPC conversations.
- Favor event steps like `set_flag`, `give_item`, and `warp` to advance progression.
- Keep NPC interactions consistent: dialog when possible, events for scripted sequences.

## Items, spells, and abilities

- Items define both usage context and targeting.
- Spells and abilities must define target modes to ensure correct menu flow.
- When using tier charges, ensure `magic_slots` exist in job definitions.
- Item and equipment sell prices are currency-aware maps (`price: {"gold": 50}`), and shops only
  allow selling when their currency matches one of the item/equipment price entries.
- Shops with `currency_pool: "tracked"` or finite `stock` persist those values in saves, keyed by
  shop ID (shared across NPCs that open the same shop).

## UI configuration

- Menu subviews should keep the main menu on the left with details on the right.
- Use the same panel titles, colors, and layout patterns established in demo content.
- Only override UI files when the default behavior does not fit your content needs.

## Validation and review

Run validation early and often:

```bash
cargo run -- validate --content content/<pack_name>
```

Before sharing or releasing a content pack, confirm:

- Validation passes with no errors.
- All references resolve (entities, maps, dialogs, events, UI).
- Start location is reachable and playable.
- Progression flags and unlocks are consistent with dialogs and events.
- UI changes render correctly in both wide and smaller terminals.
