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

### Map rendering (exploration)

- Nethack-like glyph tiles for overworld/towns/dungeons.
- Tile definitions use single glyph + palette index + collision/zone metadata.

### Battle rendering

- Mixed-mode enemy visuals:
  - Default to glyphs for small terminals.
  - Use ASCII art sprites when terminal size is at least 110x32.
  - Auto-fallback to glyphs when space is constrained.

### Battle layout (FF homage)

- Top: battlefield (enemy visuals + player sprites).
- Bottom: command region, split into three columns:
  - Left: enemy list with target highlighting.
  - Center: command menu (Attack, Magic, Items, Run, etc.).
  - Right: party list with HP/MP/ATB/status.

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
- `maps/*.json`: map tiles, entities, triggers, encounter zones.
- `entities/*.json`: jobs, spells, items, equipment, enemies, vehicles, shops, encounters, npcs.
- `events/*.json`: scripted events and cutscenes.
- `dialog/*.json`: NPC dialog trees.
- `ui/*.json`: menu panels, progress tracking config.
- `ui/battle.json`: battle layout and panel configuration.
- `ui/title.json`: title screen layout and menu.
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

- Map triggers (`on_enter`, tile or zone triggers).
- NPC interactions (map NPC `script` event).
- Dialog actions (`start_event`, `open_shop`).
- Item effects (warp, start battle).
- Battle results (victory/defeat hooks).

### New game flow

- Title "New Game" can enqueue `rules.json` `game.start_event` for opening cutscenes.

### Vehicles

- Vehicles are entities with movement constraints.
- Vehicle unlocks controlled by event flags.
- Travel routes can require specific vehicles.

### Party creation

- FF1-style startup: name + job selection for each party member.
- Job change is disabled by default; unlockable via event flag.

### Job system

- Jobs are data-driven and can be toggled on/off per rules.json.
- Job change availability controlled by an event flag.

### Magic system

- Magic schools are data-driven (white/black in demo; expandable to blue/time/etc).
- Spells reference a school, tier, cost type (MP or tier charges), target rules, and effect.
- Enemies can include traits (e.g., undead) that drive effect resolution.

### Battle system

- Supports three timing modes:
  - Turn-based (no ATB)
  - ATB with wait (pause in menus)
  - ATB active (continues during menus)
- Enemy selection tied to list on the left column.
- Visual feedback highlights enemy in battlefield and list simultaneously.

### Progress tracking

- Configurable summary panel in the menu.
- Tracks crystal progress, cleared dungeons, and other stats.
- All tracked items are event/flag driven.

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
