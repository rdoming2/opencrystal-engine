# OpenCrystal Architecture (Draft)

This document describes the planned engine architecture, data formats, and UI layout for OpenCrystal.
It is a design reference meant to guide initial implementation.

## Related docs

- `README.md` for project overview and CLI basics.
- `SCHEMAS.md` for JSON schema details and examples.
- `CONTENT_AUTHORING_GUIDE.md` for content pack workflow.

## Goals

- Terminal-first, wide compatibility (ANSI + 256 colors), with optional modern enhancements later.
- Data-driven engine: worlds, maps, entities, rules, and events defined via JSON.
- Support multiple gameplay styles: default party, party creation, job system with unlock flags,
  turn-based and Readiness variants, and multiple worlds with vehicle travel.
- Maintain accessibility with flexible input bindings and scalable layouts.

## CLI

- `cryst play [--render=auto|wide|modern] [--content path] [--content-dir path]`
- `cryst validate [--content path] [--content-dir path]`
- `cryst new-project <name> [--path path]`
- `cryst build new <kind> <id> [--content path] [--content-dir path] [--name label] [--force]`
- `cryst build map <id> [--content path] [--content-dir path]`
- `cryst build upgrade [--content path] [--content-dir path] [--dry-run]`
- `cryst build new-project <name> [--path path]`
- `cryst build docs [-s|--schemas] [-a|--architecture] [-c|--content-authoring] [-j|--jobs]`

`cryst build new` kinds: `spell`, `ability`, `item`, `equipment`, `enemy`, `vehicle`, `shop`,
`npc`, `encounter`, `job`.

`cryst play` opens a content chooser when `--content` is omitted, listing subfolders under
`--content-dir` (defaults to `~/.local/share/opencrystal/content`, `XDG_DATA_HOME/opencrystal/content`, or `%LOCALAPPDATA%\opencrystal\content` on Windows)
and showing `rules.json` metadata (title, optional description/author).

Save data is stored under `opencrystal/saves/<slugified game title>`. If the title is empty or a
legacy save folder exists using the content directory name, the engine falls back to that folder.

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
- Doors, puzzles, and campfires render as map objects, block movement, and trigger interactions on confirm.
- Puzzles can be interacted with from an adjacent tile or while standing on the puzzle tile.
- Optional death markers render on passable tiles when enabled in `rules.json` and can be hidden via Settings.

### Area name popup

- On entering a map, show the map name as a tooltip that disappears on movement.
- Controlled by `maps/*.json` `hide_name` (default false).

### Battle rendering

- Mixed-mode enemy visuals:
  - Default to glyphs for small terminals.
  - Use ASCII art sprites when terminal size is at least 110x32.
  - Auto-fallback to glyphs when space is constrained.
- Enemy list names can be colorized by HP thresholds via `ui/battle.json`.

### Title screen

- Load is disabled when no loadable saves exist; default selection is New.
- When loadable saves exist, default selection is Load.

### Battle layout

- Top: battlefield (enemy visuals + player sprites arranged in a right-side vertical grid).
- Bottom: command region, split into three columns:
  - Left: enemy list with target highlighting.
  - Center: command menu (Attack, Magic, Abilities, Items, Run, etc.).
- Right: party list with HP/MP/Readiness/status.
- Battle dialog can float at the top as a log overlay.

### Overworld map view

- The overworld map menu shows a downsampled view of the base overworld map sized
  to the current viewport.

### Build-time map editor

- `cryst build map <id>` opens a TUI editor (left map, right details) for editing tiles,
  legend entries, and map objects.
- Visual selection is rectangular (anchored) and supports yank/paste plus undo/redo.
- Objects can be added or edited in-place at the cursor, with prompts for related IDs.
- Resize prompts for anchor placement and warns about objects that will be removed;
  encounter zones are truncated and included in the warnings.

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
- `effects.json`: effect, status, trait, and element definitions used in battle resolution.
- `worlds.json`: world list, world-to-world travel rules, zoom config.
- `maps/*.json`: map tiles, entities, triggers, encounter zones, per-map encounter rate, and looped edges.
- `party.json`: predefined party roster (optional in create mode).
- `entities/*.json`: jobs (including battle sprites, starting gear, and optional magic tier charge tables), spells, abilities (optional costs), items, equipment, enemies, vehicles, shops, encounters, npcs, quests.
- `events/*.json`: scripted events and cutscenes.
- `dialog/*.json`: NPC dialog trees.
- `cooking.json`: campfire recipe sets and cooking outputs.
- `ui/*.json`: menu panels, gameplay stats config.
- `ui/battle.json`: battle layout and panel configuration.
- `ui/title.json`: title screen layout, menu, and optional logo palettes.
- `ui/menu.json`: main menu layout, optional entries, and panel templates.
- `ui/dialog.json`: dialog box layout and behavior.
- `ui/strings.json`: localization strings for UI and battle messaging.
- `input.json`: key bindings.
- `stats.json`: base and derived stat definitions.
- `save.json`: runtime save format.
- Inventory ordering is persisted in save data to keep item/equipment menu order stable across loads.

## Key systems

### World system

- Supports multiple worlds (space/time/dimensions) 
- Each world contains maps, travel rules, and zoom config.
- Worlds define `overworld_map_id` to anchor overworld-only travel and warp returns.
- Travel mechanisms:
  - Event/NPC travel.
  - Item-based travel (warp/escape).
  - Overworld fast travel (unlockable, accessed from the overworld map menu).
  - Vehicle travel with unlock flags.
- The overworld map menu uses a downsampled view of the base overworld map by
  default and reuses overworld transitions as destinations with optional costs.
- Demo content uses the downsampled overworld view (no overview map).
- Destinations remain hidden until the party has visited the target map; the
  map view highlights the party location and unlocked vehicles.
- Warp items return to the last recorded overworld entry unless a specific destination is provided.

### Event triggers

- Map triggers (`on_enter`, `trigger: "on_enter"`, `on_step` with zone support).
- NPC interactions (map NPC `script` event, dialog tree actions).
- Dialog actions (`start_event`, `open_shop`, `rest_party`, `learn_recipe`).
- Item effects (warp, start battle, learn_recipe).
- Spell learn events (direct grants).
- Party add/remove events (roster-driven joins/leaves; adds fill active slots, then reserve).
- Recipe unlock events (direct grants).
- Battle results (victory/defeat hooks) including a gameover flow that can retry the last battle
  or load recent saves before returning to the title screen.

Event execution is handled by `GameRuntime.apply_event_step`, which:
- Manages state changes (flag setting, item grants, item requirements/removals, warps to new maps).
- Returns `EventExecutionResult` for UI requests (dialogs, battles, shops).

### Random encounters

- Each movement step inside an encounter zone adds `encounter_rate` to an encounter meter.
- Each step applies a random jitter (0.5 to 1.5) to the rate before adding it to the meter.
- When the encounter meter reaches 1.0, a random battle triggers.
- After a battle, the encounter meter is reduced by 1.0 (remaining overflow is kept, clamped to 1.0).

### NPC interactions

- NPC interaction uses Manhattan distance (dx + dy) with a configurable `interaction_range` (default: 1).
- NPCs can be placed behind counters or other obstacles by setting `interaction_range` to 2 or higher.
- Map NPCs with `requires_flags` are hidden from rendering, collision, and interaction until all flags are set.
- Roaming NPCs may idle each tick based on `behavior.idle_chance` (0.0-1.0).
- Interaction range must be >= 1; values < 1 are rejected by validation.

### New game flow

- Title "New Game" can enqueue `rules.json` `game.start_event` for opening cutscenes.
- `party_mode` controls the opening flow:
  - `create`: Shows character creation screen with naming and job choice.
  - `preset`: Loads the preset roster from `party.json` and skips the create UI.
  - `preset_rename`: Same as `preset` but shows rename prompts for initial characters and future recruits.

### Vehicles

- Vehicles are entities with movement constraints.
- Vehicle unlocks controlled by event flags.
- Travel routes can require specific vehicles.
- Overworld vehicle placements live in map data and render using optional glyph/palette.

### Party creation

- Party creation modes (`party_mode`):
  - `create`: name + job selection; job list filtered by job `unlock_flag` and preselects
    the job marked `is_default`. If `systems.jobs` is disabled, job selection is skipped.
  - `preset`: roster-driven party that skips menu-based creation.
  - `preset_rename`: same as `preset` but immediately offers rename prompts for new characters.
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
- Status menu EXP display follows progression mode: character/job_points use `actor.exp`, job mode uses current job EXP, activity hides EXP.
- Job growth can use formula or table modes per base stat.
- Derived stat formulas support `lvl` for level-based scaling.
- Activity mode uses proficiency growth (weapon/magic) with soft caps and rank labels.
- Activity mode can apply post-battle stat growth using `stats.json` growth formulas
  and `rules.json` activity_growth tuning.

### Inventory + equipment

- Inventory stacks are seeded from `rules.json` `inventory` and capped by `max_stack`.
- Equipment is slot-based (weapons, armor, accessories) and validated by job categories.
- Equipping recomputes derived stats and clamps current HP/MP to new maxima.
- Equipment can grant spells or abilities while equipped.

### Shops

- Shops support buy/sell flows with category filtering and list paging in the TUI.
- Inventory entries can be infinite or finite stock; sold items can disappear or enter merchant stock.
- Merchants can use infinite funds or a tracked currency pool that persists in saves (tracked by shop ID).
- Sell pricing uses item/equipment base prices keyed by currency, with shop-level and per-entry overrides.

### Cooking system

- Campfire recipe selection shows a left-hand recipe list with a right-hand detail panel.
- The detail panel lists ingredient requirements alongside current inventory counts.

### Magic system

- Magic schools are data-driven (white/black in demo; expandable to blue/time/etc).
- Spell unlocks are configured per job.
- Magic acquisition can be `level`, `item` (spellbooks teach), `equip` (magic items grant spells while equipped), or `jp`.
- Ability acquisition can be `level`, `item`, `equip`, or `jp`.
- Magic Equip slots are defined per job progression and use equipment `slot: "magic"` for equippable spell items.
- Magic system mode can switch between MP and tier charges.
- Menu casting supports field-friendly spells (heal/revive); damage stays battle-only.
- Tier charges are tracked per character and shared across spells of the same tier.
- Global shared MP pools are a future magic style to add.

### Ability system

- Abilities are data-driven and unlocked through job progression.
- Abilities are battle-only and do not consume MP.

- Enemies can include traits (e.g., undead) that drive effect resolution.

### Battle system

- Supports three timing modes:
- Turn-based (no Readiness)
- Dynamic Wait (Readiness with pause in menus)
- Dynamic (Readiness active, continues during menus)
- Turn order ranks all actors by speed each round (party + enemies).
- Battle pause toggles a PAUSED overlay and freezes battle progression until resumed.
- Enemy selection tied to list on the left column.
- Visual feedback highlights enemy in battlefield and list simultaneously.
- Victory flow grants EXP/loot/currency stacks summed from enemies.
- Level-up modals can list newly learned spells/abilities; activity mode shows growth modals after victory.
- Fallen party members earn EXP/JP only when `battle.exp_for_fallen` is enabled.
- Battle commands come from `rules.json` `battle.commands` plus job/secondary-job additions.
- The global command list defines the base menu; job commands augment it without duplicates.
- Command ordering follows `sort_order`, and the command panel pages when the list overflows.
- Optional front/back row rules can reduce physical damage in back row and reduce melee damage unless
  using a ranged weapon category; row switching can be enabled as a battle command.
- Status effects and traits are resolved via `effects.json` definitions (poison ticks, protect/shell, elemental multipliers).
- Spells and abilities declare target modes (`single`/`multi`) with optional attenuation for group targeting.
- Statuses can be configured to clear at battle end; poison also ticks on overworld movement steps.
- Battle formulas can be configured for hit, crit, and damage calculations; boss scaling is optional and gated by the `boss` trait.
- Difficulty scale settings apply a multiplier to all enemy stats and stack with boss scaling.
- Difficulty scale can optionally scale EXP and currency rewards when enabled in battle rules.

### Progress tracking

- Configurable summary panel in the menu.
- Tracks crystal progress, cleared dungeons, and other stats.
- All tracked items are event/flag driven.
- Gameplay stats panel is shown via the Gameplay Stats menu entry.
- Stat updates can be emitted via event steps (`stat_set`, `stat_add`, `stat_max`).
- Progress stats are persisted in saves; `time_played` is derived from playtime.

### Main menu

- Two-pane layout: left list of entries, right detail pane.
- Default right pane shows party/status summary until a submenu is confirmed.
- Confirm moves focus to the right pane (submenu content); Cancel returns to list.
- Menu is modal and pauses overworld updates.
- The stats strip shows playtime, non-zero currency balances, and the current map coordinates.
- Menu entries are optional and can be gated by rules `systems` toggles and optional
  unlock flags (e.g., Summons, Job Change, Journal, Save).
- Party submenu exposes actions per member (reorder, swap with reserve, switch row). Swap actions are
  gated by save-allowed maps; row selection is shown when battle row rules are enabled.
- Custom status/gameplay stats panels are handled via configurable menu panels in `ui/menu.json`.
- Magic Equip is an optional submenu for equipping spell-granting items.
- Settings submenu exposes user options (autosave, readiness speed, difficulty scale, battle mode) that can
  be hidden or locked via `rules.json` `settings` definitions.

### Journal system

- Optional journaling/quest tracking via rules.json (`systems.journal`).
- Quest definitions loaded from `entities/quests.json` with categories and step trees.
- Categories define display labels and sort order (e.g., main, side, faction, bounty).
- Quests reference a category and contain ordered steps with optional substeps.
- Quest progress tracked via flags using `quest.<quest_id>.<step_id>` naming convention.
- Quest visibility: a quest appears only when its first step flag is set (quest acquired).
- Step completion: a step is complete when its flag is set.
- Step visibility: completed steps always show once the quest is acquired.
- Step visibility: when `show_flag` is present, incomplete steps stay hidden until that flag is set.
- Step visibility: when `show_flag` is absent, incomplete steps are revealed after the previous step in the same list is complete.
- History entries derived from completed steps in the order they appear in the quest definition.
- Substeps follow the same visibility/completion rules as parent steps.
- Flag namespaces: `quest.*` for quests, `map.*` for map state, `system.*` for features, `vehicle.*` for vehicles, `dungeon.*` for dungeons.

### Save system

- JSON saves with versioning fields for future obfuscation.
- Use a reserved field for `encoding` (e.g., "plain") to allow future formats.
- Save data includes map state (flags + entity state), active vehicle, and vehicle positions.
- Autosave writes to slot 0 after every map transition when enabled.
- Save files live under the user data directory (`~/.local/share/opencrystal/saves/<content>/`).
- Title "Load" opens a slot picker and restores the runtime state.

## Module layout (suggested)

- `crates/engine/`
  - `world`: world state, maps, travel rules
  - `entities`: jobs, items, equipment, spells, enemies, vehicles
- `battle`: turn/Readiness systems, damage, status
  - `events`: event runner, triggers, flags
  - `save`: serialization and schema versions
  - `rules`: rules loader and validation
- `crates/tui/`
  - `app`: re-exports for public API
  - `battle`: battle rendering and summary modals
  - `dialog`: dialog boxes, prompts, and choice selections
  - `input`: key mapping and action routing
  - `layout`: dynamic terminal sizing and layout logic
  - `menu`: main menu and inventory rendering
  - `overworld`: map rendering and overworld dialogs
  - `renderer`: core rendering modes (auto, wide, modern)
  - `session`: terminal lifecycle and session management
  - `shop`: shop interface
  - `title`: title screen and menu
  - `ui`: UI file loading and schema definitions
  - `utils`: shared rendering and layout utilities
- `crates/cli/src/`
  - `main`: CLI entry point and command routing
  - `utils`: common utilities (e.g., `read_action`)
  - `shop`: shop interface and item lookup
  - `party`: party creation flow and initialization
  - `dialog`: dialog execution engine (TUI and console modes)
  - `events`: event loop execution and result handling
  - `overworld`: map navigation, NPCs, and world interaction
  - `menu/`: comprehensive menu system
    - `mod.rs`: main menu loop orchestration
    - `common.rs`: shared structs (InventoryEntry, SpellEntry, etc.)
    - `status.rs`: status panel rendering
    - `inventory.rs`: item management and usage
    - `equipment.rs`: equipment management and stat previews
    - `magic.rs`: spell casting and MP/charge management
    - `magic_equip.rs`: magic equip slots and spell items
    - `abilities.rs`: ability system
  - `battle/`: complete battle system
    - `mod.rs`: main battle loop and encounters
    - `state.rs`: battle state management and turn phases
    - `logic.rs`: turn order, enemy AI, battle logging
    - `actions.rs`: action execution (attack, magic, ability, use item)
    - `render.rs`: battle UI render state construction
    - battle command routing hides abilities assigned to available `abilities_group` or `ability_id` commands from the generic Abilities list
- `crates/content/`
  - demo content and templates

## JSON schema sketch (high-level)

### rules.json

```json
{
  "version": 1,
  "game": {
    "title": "OpenCrystal",
    "party_size": 4,
    "party_reserve_size": 4,
    "battle_mode": "dynamic",
    "readiness_speed": 2.0,
    "magic_system": "mp",
    "magic_acquisition": "level",
    "ability_acquisition": "level",
    "start_event": "intro_cutscene",
    "start_location": {
      "world": "gaia",
      "map": "overworld_gaia",
      "x": 20,
      "y": 14
    },
    "currencies": [{"id": "gold", "name": "Gold", "symbol": "G"}]
  },
  "party_mode": "create",
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
    "fast_travel": true,
    "overworld_map": true,
    "save": true,
    "settings": true,
    "summons": false,
    "magic_equip": false
  },
  "save": {
    "slots_max": 10
  },
  "progression_mode": "job",
  "activity_progression": {
    "weapon_gain": {"attack": 0.02, "ability": 0.03, "cast": 0.0},
    "magic_gain": {"attack": 0.0, "ability": 0.0, "cast": 0.02},
    "effects": {"damage_scale": 0.25, "hit_bonus": 0.15},
    "unarmed_category": "unarmed",
    "ranks": [
      {"min": 0.0, "label": "Novice"},
      {"min": 0.2, "label": "Skilled"},
      {"min": 0.5, "label": "Veteran"},
      {"min": 0.8, "label": "Master"}
    ]
  },
  "activity_growth": {
    "base_rate": 0.35,
    "min_gain_threshold": 0.25,
    "min_battle_turns": 1,
    "danger_factor_min": 0.25,
    "danger_factor_max": 2.0,
    "floor_depth_exponent": 0.0,
    "status_effect_weight": 1.0,
    "initiative_weight": 0.0,
    "combo_weight": 0.0,
    "survival_bonus": 1.2,
    "soft_caps": {
      "hp": 180,
      "mp": 90,
      "str": 45,
      "int": 45,
      "vit": 45,
      "agi": 45,
      "lck": 45
    }
  },
  "settings": {
    "autosave_enabled": {"value": true, "visible": true, "editable": true},
    "readiness_speed": {
      "value": 2.5,
      "min": 0.5,
      "max": 5.0,
      "step": 0.5,
      "visible": true,
      "editable": true
    },
    "difficulty_scale": {
      "value": 1.0,
      "min": 0.5,
      "max": 2.0,
      "step": 0.1,
      "visible": true,
      "editable": true
    },
    "battle_mode": {
      "value": "dynamic_wait",
      "options": ["dynamic_wait", "dynamic"],
      "visible": true,
      "editable": true
    }
  },
  "job_system": {
    "secondary_jobs": true,
    "jp_mode": "earn",
    "job_exp_curve": {
      "mode": "table",
      "table": [0, 5, 15, 30, 50],
      "max_level": 5
    }
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
      "overworld_map_id": "overworld_gaia",
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

- Rune stone arc (4 elemental dungeons + bonus island dragon finale).
- Overworld + 1 town + 5 dungeons (4 rune sites + bonus).
- Small farm location with an ingredient merchant and ambient NPCs.
- 10-15 enemies, 5 jobs, 6-8 spells, 10 items.
- Job unlocks tied to boss NPC encounters; airship unlock after all four runes.
- Jobs use per-job leveling, so changing jobs updates the actor level to the tracked job level.
- Boss encounters trigger from NPC dialog with post-battle flash/vanish (dragon remains).
- Boss clear events add narration lines for job unlocks.
