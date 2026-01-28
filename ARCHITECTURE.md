# OpenCrystal Architecture (Draft)

This document describes the planned engine architecture, data formats, and UI layout for OpenCrystal.
It is a design reference meant to guide initial implementation.

## Goals

- Terminal-first, wide compatibility (ANSI + 256 colors), with optional modern enhancements later.
- Data-driven engine: worlds, maps, entities, rules, and events defined via JSON.
- Support multiple gameplay styles: FF1-style party creation, job system with unlock flags,
  turn-based and ATB variants, and multiple worlds with vehicle travel.
- Maintain accessibility with flexible input bindings and scalable layouts.

## CLI

- `cryst play [--render=auto|wide|modern]`
- `cryst validate`
- `cryst new-project`
- `cryst build`

## Rendering & UI

### Rendering tiers

- Wide (default): ANSI + 256 colors, box drawing, Unicode blocks. No truecolor required.
- Modern (optional): truecolor and richer effects if detected or forced via CLI.

### Color strategy

- Use semantic palette indices instead of fixed RGB values.
- Allow terminal themes (Catppuccin, Ristretto, etc.) to define the mood.
- Palette names map to ANSI colors (`red`, `green`, `blue`, etc.) with `bright_*` variants
  for highlights; monochrome modes ignore palette styling.

### Map rendering (exploration)

- Nethack-like glyph tiles for overworld/towns/dungeons.
- Tile definitions use single glyph + palette index + collision/zone metadata.
- Transition markers can override glyphs and palettes to highlight exits.
- NPCs render with per-definition palette overrides (theme-driven colors).
- Signs render as map objects with a glyph override (default `⚑`), block movement, and open a centered dialog on confirm.
- Chests render with closed/open glyphs, block movement, and display loot in a centered dialog.

### Area name popup

- On entering a map, show the map name as a tooltip that disappears on movement.
- Controlled by `maps/*.json` `hide_name` (default false).

### Battle rendering

- Mixed-mode enemy visuals:
  - Default to glyphs for small terminals.
  - Use ASCII art sprites when terminal size is at least 110x32.
  - Auto-fallback to glyphs when space is constrained.

### Battle layout (FF homage)

- Top: battlefield (enemy visuals + player sprites arranged in a right-side vertical grid).
- Bottom: command region, split into three columns:
  - Left: enemy list with target highlighting.
  - Center: command menu (Attack, Magic, Abilities, Items, Run, etc.).
  - Right: party list with HP/MP/ATB/status.
- Battle dialog can float at the top as a log overlay.

### Overworld map zoom

- Two discrete zoom levels:
  - Overview: full world map if space allows, otherwise scrollable.
  - Exploration: standard overworld LOD.

## Input

Default bindings (configurable via input.json):

- Movement: Arrow keys, WASD, HJKL
- Confirm: Enter/Return, E
- Cancel: C
- Menu: I, Esc
- Battle pause: Space (shows PAUSE overlay)

## Data model overview

All runtime data is loaded from JSON. Files are organized into top-level categories:

- `rules.json`: global rules and toggles.
- `worlds.json`: world list, world-to-world travel rules, zoom config.
- `maps/*.json`: map tiles, entities, triggers, encounter zones, per-map encounter rate.
- `party.json`: predefined party roster (optional in create mode).
- `entities/*.json`: jobs (including battle sprites and starting gear), spells, abilities, items, equipment, enemies, vehicles, shops, encounters, npcs.
- `events/*.json`: scripted events and cutscenes.
- `dialog/*.json`: NPC dialog trees.
- `ui/*.json`: menu panels, progress tracking config.
- `ui/battle.json`: battle layout and panel configuration.
- `ui/title.json`: title screen layout and menu.
- `ui/menu.json`: main menu layout, optional entries, and panel templates.
- `ui/dialog.json`: dialog box layout and behavior.
- `input.json`: key bindings.
- `stats.json`: base and derived stat definitions.
- `save.json`: runtime save format.

## Key systems

### World system

- Supports multiple worlds (classic FF-style world jumps).
- Each world contains maps, travel rules, and zoom config.
- Travel mechanisms:
  - Event/NPC travel.
  - Item-based travel (warp/escape).
  - Overworld fast travel (unlockable).
  - Vehicle travel with unlock flags.

### Event triggers

- Map triggers (`on_enter`, `trigger: "on_enter"`, `on_step` with zone support).
- NPC interactions (map NPC `script` event, dialog tree actions).
- Dialog actions (`start_event`, `open_shop`).
- Item effects (warp, start battle).
- Battle results (victory/defeat hooks).

Event execution is handled by `GameRuntime.apply_event_step`, which:
- Manages state changes (flag setting, item grants, warps to new maps).
- Returns `EventExecutionResult` for UI requests (dialogs, battles, shops).

### New game flow

- Title "New Game" can enqueue `rules.json` `game.start_event` for opening cutscenes.

### Vehicles

- Vehicles are entities with movement constraints.
- Vehicle unlocks controlled by event flags.
- Travel routes can require specific vehicles.

### Party creation

- Party creation modes (`party_mode`):
  - `create`: name + job selection; job list filtered by job `unlock_flag` and preselects
    the job marked `is_default`. If `systems.jobs` is disabled, job selection is skipped.
  - `predefined`: roster-driven party (FF5/FF6 style) with optional job menu.
- Experience curves are configurable via `exp_curve` (table or formula).
- Derived stat formulas can reference `lvl` for level scaling.
- Job change is disabled by default; unlockable via event flag.
- Future slot rules (e.g., dual wield constraints) can extend job equipment slots.
- Renaming is planned as a reusable menu/action for story joins or rename NPCs.

### Job system

- Jobs are data-driven and can be toggled on/off per rules.json.
- Job change availability controlled by an event flag.

### Leveling + experience

- Experience thresholds are configured via `exp_curve` in `rules.json` (table or formula).
- Job growth can use formula or table modes per base stat.
- Derived stat formulas support `lvl` for level-based scaling.

### Inventory + equipment

- Inventory stacks are seeded from `rules.json` `inventory` and capped by `max_stack`.
- Equipment is slot-based (weapons, armor, accessories) and validated by job categories.
- Equipping recomputes derived stats and clamps current HP/MP to new maxima.

### Magic system

- Magic schools are data-driven (white/black in demo; expandable to blue/time/etc).
- Spell unlocks are configured per job.
- Magic system mode can switch between MP and tier charges.
- Menu casting supports field-friendly spells (heal/revive); damage stays battle-only.

### Ability system

- Abilities are data-driven and unlocked through job progression.
- Abilities are battle-only and do not consume MP.

- Enemies can include traits (e.g., undead) that drive effect resolution.

### Battle system

- Supports three timing modes:
  - Turn-based (no ATB)
  - ATB with wait (pause in menus)
  - ATB active (continues during menus)
- Turn order ranks all actors by speed each round (party + enemies).
- Enemy selection tied to list on the left column.
- Visual feedback highlights enemy in battlefield and list simultaneously.
- Victory flow grants EXP/loot/currency summed from enemies.

### Progress tracking

- Configurable summary panel in the menu.
- Tracks crystal progress, cleared dungeons, and other stats.
- All tracked items are event/flag driven.

### Main menu

- Two-pane layout: left list of entries, right detail pane.
- Default right pane shows party/status summary until a submenu is confirmed.
- Confirm moves focus to the right pane (submenu content); Cancel returns to list.
- Menu is modal and pauses overworld updates.
- Menu entries are optional and can be gated by rules `systems` toggles and optional
  unlock flags (e.g., Summons, Materia, Job Change, Journal, Save).
- Custom status/progress panels are handled via configurable menu panels in `ui/menu.json`.

### Journal system

- Optional journaling/quest tracking via rules.json.
- Journal entries driven by events.
- Quest resolver maps flags to quest step visibility/completion based on a defined convention.

### Save system

- JSON saves with versioning fields for future obfuscation.
- Use a reserved field for `encoding` (e.g., "plain") to allow future formats.
- Save data includes map state (flags + entity state) and global entities (vehicles).

## Module layout (suggested)

- `crates/engine/`
  - `world`: world state, maps, travel rules
  - `entities`: jobs, items, equipment, spells, enemies, vehicles
  - `battle`: turn/ATB systems, damage, status
  - `events`: event runner, triggers, flags
  - `save`: serialization and schema versions
  - `rules`: rules loader and validation
- `crates/tui/`
  - `renderer`: map rendering, battle rendering
  - `layout`: dynamic terminal sizing, battle layout
  - `input`: key mapping and action routing
- `crates/cli/`
  - `commands`: play/validate/new-project/build
- `crates/content/`
  - demo content and templates

## JSON schema sketch (high-level)

### rules.json

```json
{
  "version": 1,
  "game": {
    "title": "OpenCrystal",
    "start_mode": "ff1",
    "party_size": 4,
    "party_reserve_size": 4,
    "battle_mode": "turn",
    "magic_system": "mp",
    "job_change_enabled": false,
    "job_change_flag": "world.job_change_unlocked",
    "currency": {"id": "gil", "name": "G", "symbol": "G"}
  },
  "party_mode": "predefined",
  "exp_curve": {
    "mode": "table",
    "table": [0, 10, 30, 60, 100],
    "max_level": 5
  },
  "inventory": {
    "max_stack": 99,
    "items": [{"id": "potion", "qty": 5}],
    "equipment": [{"id": "bronze_sword", "qty": 1}]
  },
  "systems": {
    "items": true,
    "magic": true,
    "equipment": true,
    "status": true,
    "party": true,
    "jobs": true,
    "journal": true,
    "save": true,
    "settings": true,
    "summons": false,
    "materia": false
  },
  "magic_tiers": [
    {"tier": 1, "max_charges": 3},
    {"tier": 2, "max_charges": 2},
    {"tier": 3, "max_charges": 1}
  ],
  "features": {
    "journal": true,
    "fast_travel": true,
    "overworld_map": true
  },
  "render": {
    "min_art_width": 110,
    "min_art_height": 32
  }
}
```

### worlds.json

```json
{
  "version": 1,
  "worlds": [
    {
      "id": "gaia",
      "name": "Gaia",
      "starting_map": "overworld_gaia",
      "zoom_levels": ["overview", "explore"],
      "overview": {"enabled": true, "map_id": "gaia_overview"},
      "vehicles": ["ship", "airship"],
      "fast_travel": {"enabled": true, "requires_flag": "world.fast_travel_unlocked"},
      "links": [{"to_world": "luna", "requires_flag": "world.luna_unlocked"}]
    }
  ]
}
```

### input.json

```json
{
  "version": 1,
  "bindings": {
    "move_up": ["Up", "W", "K"],
    "move_down": ["Down", "S", "J"],
    "move_left": ["Left", "A", "H"],
    "move_right": ["Right", "D", "L"],
    "confirm": ["Enter", "E"],
    "cancel": ["C"],
    "menu": ["I", "Escape"],
    "pause": ["Space"]
  }
}
```

## Demo content

- Crystal-focused story (2 of 4 crystals recovered in demo).
- Overworld + 1 town + 1 dungeon.
- 6-8 enemies, 3-4 jobs, 6-8 spells, 10 items.
- Job unlock event and vehicle unlock event.
