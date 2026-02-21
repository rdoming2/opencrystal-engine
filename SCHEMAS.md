# OpenCrystal JSON Schemas (Draft)

This document defines the core JSON formats for OpenCrystal content. These are draft schemas
intended to guide implementation and are subject to change as the engine matures.

All files are UTF-8 JSON. Each top-level schema includes a `version` field for forward
compatibility.

## Related docs

- `README.md` for project overview and CLI basics.
- `CONTENT_AUTHORING_GUIDE.md` for content pack workflow and conventions.

## Shared conventions

- All IDs are lowercase snake_case strings.
- All entity references use IDs, not filenames.
- Missing optional fields should fall back to documented defaults unless noted as required.
- Palette values use terminal color names (`red`, `green`, `blue`, etc.) plus bright variants
  (`bright_red`, `bright_green`, etc.). These map to the user's terminal theme.

## Builder tooling

`cryst build new` can stub schema entries (spells, items, jobs, etc.) under an existing content
pack. It does not invent missing required context; for example, spell creation requires at least
one magic school in `entities/spells.json`.

Supported kinds: `spell`, `ability`, `item`, `equipment`, `enemy`, `vehicle`, `shop`, `npc`,
`encounter`, `job`.

Example:

```bash
cryst build new job ranger --content content/demo
```

`cryst build docs` prints schema and design references to stdout for automation and LLM workflows.

## input.json

Configurable input bindings.

```json
{
  "version": 1,
  "bindings": {
    "move_up": ["Up", "W", "K"],
    "move_down": ["Down", "S", "J"],
    "move_left": ["Left", "A", "H"],
    "move_right": ["Right", "D", "L"],
    "confirm": ["Enter", "C"],
    "cancel": ["X"],
    "menu": ["I", "Escape"],
    "pause": ["Space"],
    "quit": ["Q"]
  }
}
```

The `pause` binding toggles battle pause during combat.

## rules.json

Global rules and feature flags.

`party_mode` controls the opening flow:
- `create`: Present the character creation UI with naming and job selection.
- `preset`: Load the preset roster from `party.json` and skip the create UI.
- `preset_rename`: Same as `preset` but prompts to rename starting characters (and future recruits) through the rename helper.
`party_create` provides defaults for create mode (starting level and name length).
Job selection always shows unlocked jobs; `systems.jobs` only gates job-related UI (job menu/change screens) and does not disable job usage entirely.

`exp_curve` defines the XP thresholds for leveling. Use `mode: "table"` with
absolute XP totals per level or `mode: "formula"` with a formula string (use
`lvl` in the formula). `max_level` caps progression.
Status menu EXP uses `exp_curve` for `character`/`job_points` progression and
`job_system.job_exp_curve` for `job` progression.

`inventory` seeds starting inventory and sets `max_stack` for items/equipment.
`magic_acquisition` defines how spells are obtained (`level`, `item`, `equip`, `jp`) and is required.
`ability_acquisition` defines how abilities are obtained (`level`, `item`, `equip`, `jp`) and is required.
`job_system.jp_mode` controls whether JP is spent (`spend`) or earned (`earn`, `earn_job_locked`) and is required.
`battle` defines the command catalog and the global command set used in battle. The global set is
the default menu for every job; job `commands` entries add to it (primary + secondary jobs are
merged, duplicates removed). Command labels come from the catalog so UI text can be overridden.
`battle.exp_for_fallen` toggles whether fallen party members earn EXP/JP after victory.
`battle.formulas` configures hit, crit, and damage calculations (expressions with stat variables).
`battle.boss_scaling` enables optional stat multipliers for enemies with the `boss` trait.
`battle.rows` enables optional front/back row rules; when enabled, back row reduces physical attack
unless using a ranged weapon category and reduces incoming physical damage.
`systems` toggles whether a gameplay system/menu is enabled at all. It should be
used for global availability (e.g., disabling magic equip or gameplay stats). Menu entries
can still add `unlock_flag` gating for progression-driven unlocks.
Set `systems.cooking` to enable campfire cooking.
`settings` configures user-facing settings. Each setting can be hidden
(`visible: false`), locked (`editable: false`), or provide allowed values. For
`readiness_speed`, `min`, `max`, and `step` define the enforced range; for
`battle_mode`, `options` lists the allowed choices (leave empty to lock to `value`).
`game.description` and `game.author` are optional metadata used for content selection screens.
Tier charges are defined within each job's `magic_slots`; `magic_system` `tier_charges` uses those definitions per actor.
`save` configures slot count. `slots_max` counts manual slots
only; autosave uses slot 0 and never reduces the manual slot total.

```json
{
  "version": 1,
  "game": {
    "title": "OpenCrystal",
    "description": "A crystal-bound journey",
    "author": "OpenCrystal Team",
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
  "battle": {
    "exp_for_fallen": false,
    "global_commands": ["attack", "defend", "items", "run"],
    "commands": [
      {"id": "attack", "label": "Attack", "kind": "attack", "sort_order": 10},
      {"id": "magic", "label": "Magic", "kind": "magic", "sort_order": 30},
      {"id": "abilities", "label": "Abilities", "kind": "abilities", "sort_order": 40},
      {"id": "items", "label": "Items", "kind": "items", "sort_order": 50},
      {"id": "defend", "label": "Defend", "kind": "defend", "sort_order": 60},
      {"id": "run", "label": "Run", "kind": "run", "sort_order": 70}
    ],
    "rows": {
      "enabled": true,
      "allow_battle_switch": true,
      "back_row_attack_multiplier": 0.5,
      "back_row_defense_multiplier": 0.5,
      "ranged_weapon_categories": ["bow", "crossbow", "gun"],
      "battle_shift": 1
    }
  },
  "party_mode": "create",
  "party_create": {
    "starting_level": 1,
    "name_length": 12
  },
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
    "gameplay_stats": true,
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
    "battle_mode": {
      "value": "dynamic_wait",
      "options": ["dynamic_wait", "dynamic"],
      "visible": true,
      "editable": true
    }
  },
  "job_system": {
    "progression_mode": "job",
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
    "min_art_height": 32,
    "palette": "terminal"
  },
  "stats": {
    "track": [
      "time_played",
      "enemies_defeated",
      "max_damage",
      "jobs_unlocked",
      "crystals_collected",
      "dungeons_cleared"
    ]
  }
}
```

## job_system

Describes how the job system tracks progression beyond the standard
experience curve. Put this object in `rules.json` alongside `exp_curve`
to enable job-specific leveling, JP spending, and optional secondary jobs.

- `progression_mode` selects `character` (baseline XP), `job` (per-job
  XP/levels), or `job_points` (global XP plus job-specific JP).
- `job_exp_curve` defines thresholds when `job` mode is active.
- `secondary_jobs` toggles the secondary job slot exposed by the Job
  menu.

Spells and abilities can declare `unlock_level` or `jp_cost` inside the
job definitions so that the menu and progression logic know whether they
unlock automatically or need JP spending.

## battle (rules.json)

Battle command catalog and default global command set.

- `exp_for_fallen`: allow fallen party members to earn EXP/JP after victory.
- `global_commands`: list of command IDs available to every job.
- `commands`: list of command definitions.

Command definition fields:

- `id`: unique command ID (lowercase snake_case).
- `label`: UI label for the command.
- `kind`: `attack`, `magic`, `abilities`, `abilities_group`, `items`, `run`, `defend`, `row`.
- `sort_order`: order in the command list (lower values first).
- `ability_group`: required when `kind` is `abilities_group`; matches abilities `command_group`.
- `ability_id`: optional when `kind` is `abilities`; routes the command directly to an ability.
- `abilities` commands hide abilities routed to available `abilities_group` commands or `ability_id` commands.

Battle formula fields:

- `formulas.physical`: expression for physical base damage.
- `formulas.magic`: expression for magic base damage.
- `formulas.hit`: expression for hit chance (0-1).
- `formulas.crit`: expression for crit chance (0-1).
- `formulas.crit_multiplier`: damage multiplier applied on crit.

Formula variables:

- `atk`, `def`, `matk`, `mdef`, `agi`, `lck`, `eva`, `lvl`
- `target_eva`, `target_lvl`, `power`

Boss scaling fields:

- `boss_scaling.enabled`: enable scaling for enemies with `boss` trait.
- `boss_scaling.hp_multiplier`: multiplier for enemy HP.
- `boss_scaling.stat_multiplier`: multiplier for other stats.

Battle row fields:

- `enabled`: toggles front/back row rules.
- `allow_battle_switch`: allow row switching via the battle command.
- `back_row_attack_multiplier`: multiplier applied to physical attack when in back row (<= 1).
- `back_row_defense_multiplier`: multiplier applied to physical damage taken in back row (<= 1).
- `ranged_weapon_categories`: equipment categories that ignore the back-row attack penalty.
- `battle_shift`: horizontal grid shift for back-row sprites in battle rendering.

## party.json

Defines the roster used in `predefined` party mode. For `create` mode, this file
is optional and can be omitted. `base_stats` keys should match `stats.json` base
stat IDs. `starting_equipment` is an optional override; jobs can define default
starting gear.

```json
{
  "version": 1,
  "roster": [
    {
      "id": "alric",
      "name": "Alric",
      "job_id": "fighter",
      "level": 1,
      "base_stats": {"hp": 32, "mp": 0, "str": 8, "int": 2},
      "spells": ["cure"]
    }
  ],
  "starting_party": ["alric"],
  "reserve": []
}
```

## effects.json

Defines effect, status, trait, and element definitions. This file is top-level (sibling to
`rules.json`). Effects are hardcoded by `kind` in the engine, but parameters live here so
content can reference them by ID.

Effect kinds (initial):

- `apply_status`: applies a status by id to the target.
- `poison_tick`: deals damage at the start of the target's turn.
- `skip_turn`: prevents action on the target's turn (chance-based).
- `immobile`: prevents action on the target's turn.
- `damage_multiplier`: multiplies incoming damage (`damage_kind`: `physical`, `magic`, `all`).
- `element_multiplier`: multiplies damage for a specific element.
- `healing_inversion`: converts healing into damage for the target.

Status fields:

- `default_duration`: turn count (<= 0 means infinite).
- `reapply`: `refresh`, `stack`, or `ignore`.
- `tick`: `turn_start` (default).
- `clear_on_battle_end`: whether the status is removed after battle ends.
- `effects`: list of effect IDs applied while the status is active.

Trait fields:

- `effects`: list of effect IDs applied while the trait is present.

```json
{
  "version": 1,
  "elements": [
    {"id": "fire", "label": "Fire"},
    {"id": "ice", "label": "Ice"},
    {"id": "lightning", "label": "Lightning"}
  ],
  "effects": [
    {
      "id": "apply_poison",
      "label": "Apply Poison",
      "kind": "apply_status",
      "status": "poison",
      "chance": 1.0
    },
    {
      "id": "poison_tick",
      "label": "Poison Tick",
      "kind": "poison_tick",
      "percent": 0.1
    },
    {
      "id": "resist_fire",
      "label": "Resist Fire",
      "kind": "element_multiplier",
      "element": "fire",
      "multiplier": 0.5
    }
  ],
  "statuses": [
    {
      "id": "poison",
      "label": "Poison",
      "short": "PSN",
      "default_duration": 4,
      "reapply": "refresh",
      "tick": "turn_start",
      "clear_on_battle_end": false,
      "effects": ["poison_tick"]
    }
  ],
  "traits": [
    {"id": "undead", "label": "Undead", "effects": ["undead_healing_inversion"]}
  ]
}
```

## worlds.json

Defines multiple worlds and inter-world travel.

- `overworld_map_id` identifies the world map used for overworld travel, warp returns, and fast
  travel. It should reference a map in `maps/*.json`.
- The menu map view uses a downsampled `overworld_map_id` view sized to the viewport.

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
      "fast_travel": {
        "enabled": true,
        "requires_flag": "world.fast_travel_unlocked"
      },
      "links": [
        {
          "to_world": "luna",
          "requires_flag": "world.luna_unlocked"
        }
      ]
    }
  ]
}
```

## maps/*.json

Map data for overworlds, towns, and dungeons.

NPCs reference `entities/npcs.json` by ID. `script` is optional; if omitted, the NPC
uses its dialog tree. Boss encounters can be triggered via NPC dialog actions that
start an event, then the event can `npc_hide` to prevent re-triggering. Map NPCs can
include `requires_flags` to hide the NPC (rendering, collision, interaction) until all
flags are set.

`hide_name` controls whether the map name tooltip is shown on entry. Defaults to false.

`loop` controls edge wrapping for the map. Set `loop.x` and/or `loop.y` to true to wrap
movement across that axis. Defaults to `{ "x": false, "y": false }`.

`encounter_rate` is the per-step base chance (0.0-1.0) used to build an encounter meter in encounter zones.
Each step applies a random jitter (0.5 to 1.5) to the rate before adding it to the meter.
When the meter reaches 1.0, a random battle triggers and the meter is reduced by 1.0.

`signs` are inline interactive objects that display a centered dialog with no speaker.

```json
{
  "version": 1,
  "id": "overworld_gaia",
  "name": "Gaia Overworld",
  "hide_name": false,
  "world": "gaia",
  "width": 64,
  "height": 48,
  "loop": {"x": true, "y": false},
  "tiles": [
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
    "~~~~..^^....~~~~..^^....~~~~....",
    "~~~~..^^....~~~~..^^....~~~~...."
  ],
  "legend": {
    "~": {"tile": "water", "passable": false, "palette": "blue"},
    ".": {"tile": "grass", "passable": true, "palette": "green"},
    "^": {"tile": "mountain", "passable": false, "palette": "bright_black"}
  },
  "encounters": [
    {
      "zone_id": "grasslands",
      "rect": [0, 0, 30, 20],
      "table": "gaia_grasslands"
    }
  ],
  "encounter_rate": 0.08,
  "events": [
    {
      "id": "intro",
      "trigger": "on_enter",
      "script": "intro_scene"
    },
    {
      "id": "danger_zone",
      "trigger": "on_step",
      "zone": "grasslands",
      "script": "bear_encounter"
    },
    {
      "id": "trap_tile",
      "trigger": "on_step",
      "pos": [15, 10],
      "script": "stepped_on_trap"
    }
  ],
  "npcs": [
    {
      "id": "elder",
      "pos": [10, 12],
      "script": "elder_dialog",
      "requires_flags": ["world.intro_complete"]
    }
  ],
  "signs": [
    {
      "id": "town_notice",
      "pos": [9, 6],
      "glyph": "⚑",
      "palette": "bright_yellow",
      "text": "North Road"
    }
  ],
  "chests": [
    {
      "id": "starter_cache",
      "pos": [11, 6],
      "glyph_closed": "▣",
      "glyph_open": "▢",
      "palette": "bright_yellow",
      "opened_flag": "town.starter_cache_opened",
      "loot": {
        "items": [{"id": "potion", "qty": 2}],
        "equipment": [{"id": "bronze_sword", "qty": 1}],
        "currency": [{"id": "gold", "amount": 50}]
      }
    }
  ],
  "allow_save": true,
  "save_points": [[12, 9]],
  "transitions": [
    {
      "id": "to_ember",
      "pos": [5, 8],
      "target_map": "dungeon_ember",
      "target_pos": [1, 1],
      "label": "Ember",
      "requires_flag": "dungeon.ember_unlocked",
      "cost": {"id": "gold", "amount": 200},
      "return_to_last": false,
      "glyph": "D",
      "palette": "bright_magenta"
    }
  ]
}
```

Saving rules:

- `allow_save` controls whether the main menu save command is enabled on the map.
- `save_points` lists coordinates where saving is always allowed, even if
  `allow_save` is false.
- `return_to_last` on transitions uses the last entry position for the current map
  when leaving (falls back to `target_map` + `target_pos` if unavailable).
- `label` is optional display text for fast-travel/map menus.
- `requires_flag` gates access to the transition until a flag is set.
- `cost` is an optional currency stack charged on fast travel.
- Fast travel destinations are only shown after the target map has been visited in-game.

Rendering notes:

- `legend.palette` sets the terminal palette name for the tile glyph.
- `transitions` may include a `glyph` override and `palette` to highlight exits.

Vehicle notes:

- `vehicles` defines overworld vehicle placements by `vehicle_id` and `pos`.
- `vehicles.requires_flags` optionally gates visibility until all flags are set.

Sign notes:

- `signs.glyph` is optional; defaults to `⚑` if omitted.
- `signs.text` appears in a centered dialog when the player confirms nearby.
- Signs block movement and are not passable.
- Signs are interactive objects defined directly on maps; they are not NPCs.

Chest notes:

- `chests.glyph_closed`/`chests.glyph_open` are optional; defaults to `▣` and `▢`.
- `chests.opened_flag` is required and is set when the chest is opened.
- Loot can include any combination of items, equipment, and currency stacks.
- Chests block movement and show a centered dialog on confirm.

Door notes:

- `doors` define locked or gated transitions.
- `doors.requires_flag` locks the door until the flag is set.
- `doors.locked_text` is shown when the door is locked (if no `locked_event`).
- `doors.locked_event` queues an event when interacting with a locked door.
- `doors.target_map`/`doors.target_pos` define an optional transition; doors without a target are passable once unlocked.
- Doors block movement when locked, and can render with a custom `glyph`/`palette`.

Puzzle notes:

- `puzzles` define interactive objects that block movement.
- `puzzles.requires_flags` gates visibility until all flags are set.
- `puzzles.text` shows a centered dialog when interacted with.
- `puzzles.event` queues an event when interacted with (optional alternative to `text`).
- `puzzles.set_flag` is set when the puzzle is interacted with (useful for simple switches).
- Puzzles can be interacted with from an adjacent tile or while standing on the puzzle tile.

Campfire notes:

- `campfires` define cooking interactables that block movement.
- `campfires.campfire_id` references a campfire set in `cooking.json`.
- `campfires.requires_flags` gates visibility until all flags are set.

## cooking.json

Campfire cooking recipes and sets. Recipes list explicit item IDs for ingredients and
produce items/equipment/currency on success.

```json
{
  "version": 1,
  "recipes": [
    {
      "id": "fish_skewer",
      "name": "Fish Skewer",
      "description": "A simple campfire skewer.",
      "ingredients": [
        {"id": "raw_fish", "qty": 1},
        {"id": "herb_bundle", "qty": 1}
      ],
      "results": {
        "items": [{"id": "fish_skewer", "qty": 1}],
        "equipment": [],
        "currency": []
      }
    }
  ],
  "campfires": [
    {"id": "inn_kitchen", "label": "Inn Kitchen", "recipes": ["fish_skewer"]}
  ]
}
```

Campfire fields:

- `id`: unique campfire set ID.
- `label`: display label used in the cooking dialog.
- `recipes`: list of recipe IDs available at the campfire.

UI notes:

- Campfire recipe selection shows ingredient requirements with current inventory counts in a right-hand detail panel.

Recipe fields:

- `id`: unique recipe ID.
- `name`: display label for the recipe.
- `description`: optional helper text (not shown in the dialog yet).
- `unlock_flag`: optional flag required to unlock the recipe; if omitted, the recipe is always available.
- `ingredients`: list of item stacks required to cook the recipe.
- `results`: items, equipment, and currency produced on success.

## entities/encounters.json

Encounter tables referenced by maps.

Entries contain an inline formation. Single-enemy encounters are just a formation with
one member. Each formation member must include a `pos` value. Positions are expressed
in a battle grid from `[0,0]` to `[9,5]` (10x6).

```json
{
  "version": 1,
  "tables": [
    {
      "id": "gaia_grasslands",
      "entries": [
        {
          "weight": 60,
          "formation": [
            {"enemy": "imp", "pos": [0, 1]},
            {"enemy": "imp", "pos": [1, 2]},
            {"enemy": "imp", "pos": [2, 3]}
          ]
        },
        {
          "weight": 40,
          "formation": [
            {"enemy": "wolf", "pos": [1, 2]},
            {"enemy": "wolf", "pos": [2, 3]}
          ]
        },
        {
          "weight": 20,
          "formation": [
            {"enemy": "imp", "pos": [0, 1]},
            {"enemy": "imp", "pos": [1, 2]},
            {"enemy": "wisp", "pos": [3, 0]}
          ]
        }
      ]
    }
  ]
}
```

## entities/npcs.json

Defines NPC metadata and behavior. Map placement is handled in `maps/*.json`.
`palette` is optional and sets the terminal color for the NPC glyph.
`interaction_range` is optional and sets the Manhattan distance at which the NPC can be interacted with (default: 1).
`behavior.idle_chance` is optional and sets the chance (0.0-1.0) that a roaming NPC stays in place
for a movement tick. It is ignored for non-roaming behavior types.

```json
{
  "version": 1,
  "npcs": [
    {
      "id": "wanderer",
      "name": "Sky Tinker",
      "sprite": "wanderer",
      "palette": "bright_magenta",
      "dialog": "sky_tinker",
      "behavior": {"type": "roam", "radius": 4, "idle_chance": 0.35, "persist": true}
    },
    {
      "id": "innkeeper",
      "name": "Coral Innkeeper",
      "sprite": "innkeeper",
      "palette": "bright_yellow",
      "dialog": "innkeeper",
      "behavior": {"type": "static", "idle_chance": 0.0},
      "interaction_range": 2
    }
  ]
}
```

Behavior types (initial): `static`, `roam`, `patrol`. For `patrol`, use `behavior.path`.
`behavior.idle_chance` is optional; it is used only for `roam`.
Set `behavior.persist` to keep NPC positions in `save.json`.

## dialog/*.json

Dialog trees for NPC conversations. Events can still script dialog for cutscenes.

Dialog text is automatically wrapped into multiple dialog boxes based on terminal width.

```json
{
  "version": 1,
  "id": "sky_tinker",
  "nodes": [
    {
      "id": "start",
      "speaker": "Sky Tinker",
      "text": "The winds shift. The crystals answer.",
      "actions": [
        {"type": "start_event", "event": "sky_tinker_intro"}
      ],
      "choices": [
        {"label": "Farewell", "next": "end"}
      ]
    },
    {"id": "end", "text": "Safe travels."}
  ]
}
```

Dialog action types:

- `start_event` (field: `event`)
- `open_shop` (field: `shop`)
- `set_flag` (field: `flag`)
- `give_item` (fields: `item`, `qty`)
- `rest_party` (no fields)
- `learn_recipe` (field: `recipe`)

Dialog choice fields:

- `label`: button text.
- `next`: node id, or `"end"` to close the dialog.
- `requires_flags`: optional list of flags required to show the choice.

## entities/jobs.json

Growth formulas can reference current base stats (e.g., `vit`, `int`) as inputs.
Growth expressions follow the same syntax as `stats.json` formulas and apply to
base stats only.

Jobs may also include flavor fields such as an optional `description` and a
list of `magic_schools`. The latter is used by the Job menu to explain what
schools a job can draw from when a secondary job occupies the same actor.

Growth modes:

- `formula`: per-level delta formulas for each base stat (all base stats required).
- `table`: absolute per-level values for each base stat (deltas computed at level-up).

```json
{
  "version": 1,
  "jobs": [
    {
      "id": "fighter",
      "name": "Fighter",
      "stats": {"hp": 30, "mp": 0, "str": 8, "int": 2},
      "growth": {
        "mode": "formula",
        "per_level": {
          "hp": "6 + vit",
          "mp": "0",
          "str": "2",
          "vit": "2",
          "agi": "1",
          "int": "0",
          "lck": "1"
        }
      },
      "equipment": {"weapons": ["sword"], "armor": ["plate"]},
      "equipment_slots": ["weapon", "armor"],
      "accessory_slots": 1,
      "can_dual_wield": false,
      "stat_modifiers": {
        "str": {"add": 2, "mult": 1.05},
        "int": {"add": -1, "mult": 0.95}
      },
      "starting_equipment": {"weapon": "bronze_sword", "armor": "bronze_armor"},
      "sprite": {"glyph": "F", "palette": "bright_cyan"},
      "art": {"lines": [" o ", "|/\\", " /\\"], "palette": "bright_cyan"},
      "is_default": true,
      "sort_order": 10,
      "spells": [],
      "abilities": [{"id": "power_strike", "level": 2}],
      "commands": ["abilities"]
    },
    {
      "id": "white_mage",
      "name": "White Mage",
      "stats": {"hp": 20, "mp": 10, "str": 2, "int": 8},
      "growth": {
        "mode": "formula",
        "per_level": {
          "hp": "3 + ROUND(RAND(-1,1))",
          "mp": "2 + ROUND(RAND(0,1) * 3)",
          "str": "0",
          "vit": "1",
          "agi": "1",
          "int": "2",
          "lck": "1"
        }
      },
      "equipment": {"weapons": ["staff"], "armor": ["robe"]},
      "equipment_slots": ["weapon", "armor"],
      "accessory_slots": 1,
      "can_dual_wield": false,
      "stat_modifiers": {
        "int": {"add": 2, "mult": 1.05},
        "str": {"add": -1, "mult": 0.9}
      },
      "spells": [
        {"id": "cure", "level": 1},
        {"id": "protect", "tier": 1}
      ]
    }
  ]
}
```

Job spell fields:

- `level`: optional unlock level used for `level` or `jp` earn modes.
- `tier`: optional spell tier used for `tier_charges` casting cost.
- `item`: optional item ID used for `item` acquisition.
- `unlock_level`: optional prereq level required before a `jp` purchase.
- `jp_cost`: optional job points cost used for purchasing the spell in
  `jp` spend mode.

Job ability fields:

- `level`: optional unlock level used for `level` or `jp` earn modes.
- `unlock_level`: optional prereq level required before a `jp` purchase.
- `jp_cost`: optional job points cost for manual unlocks in `jp` spend mode.

Job command fields:

- `commands`: optional list of battle command IDs to add to the global command set.

Job acquisition overrides:

- `acquisition.magic`: optional override for how the job gains spells.
  - Accepts a single mode (`level`, `item`, `equip`, `jp`) or a map of
    magic school -> mode to mix acquisition styles per school.
- `acquisition.abilities`: optional override for how the job gains abilities.
  - Accepts a single mode (`level`, `item`, `equip`, `jp`).

Job magic tier slots:

- `magic_slots`: optional mapping of tier -> per-level charges list.
- Each tier list is indexed by level (level 1 uses index 0).
- When present, these charges define the actor's tier limits for that job (there is no global `magic_tiers`).

Job magic equip slots:

- `magic_equip_progression`: optional mapping of level -> slot count.
  - Example: `{ "1": 1, "10": 2 }` grants 1 slot at level 1 and 2 slots at level 10.

Job starting equipment:

- `starting_equipment`: optional default equipment by slot.
- `sprite`: job battle glyph/palette.
- `art`: optional ASCII art for battle rendering (used when battle art mode allows).

Job gating fields:

- `unlock_flag`: optional flag required to choose the job.
- `is_default`: marks the job pre-selected in create mode; must be ungated.
- `sort_order`: manual ordering for job lists.

Equipment slot fields:

- `equipment_slots`: list of non-accessory slot IDs.
- `accessory_slots`: number of accessory slots (represented as `accessory_1`,
  `accessory_2`, etc).
- `can_dual_wield`: whether offhand weapons are allowed.
- `stat_modifiers`: additive/multiplicative adjustments applied per stat.

## entities/spells.json

Supports multiple magic schools.

Spell costs are flexible. Use `{"type": "mp"}` for MP-based casting or
`{"type": "tier_charges"}` for limited-per-tier systems. When
`magic_system` is `tier_charges`, spell cost types are interpreted as tier
uses (one shared charge per tier per cast).

Targeting fields:

- `target_mode`: `single`, `multi`, or `both`.
- `multi_attenuation`: optional multiplier applied per target when in multi mode.

```json
{
  "version": 1,
  "schools": [
    {"id": "white", "name": "White"},
    {"id": "black", "name": "Black"}
  ],
  "spells": [
    {
      "id": "cure",
      "name": "Cure",
      "school": "white",
      "tier": 1,
      "cost": {"type": "mp", "value": 3},
      "default_target": "ally",
      "allowed_targets": ["ally", "enemy"],
      "target_mode": "single",
      "multi_attenuation": 0.6,
      "effect": {"type": "heal", "power": 20}
    }
  ]
}
```

## entities/abilities.json

Abilities define non-MP combat techniques unlocked by jobs.

Ability costs (optional):

- `type`: `mp`, `hp`, `currency`, `item`, `death`, `random`
- `value`: numeric cost amount (ignored for `death`)
- `item_id`: required when `type` is `item`
- `currency_id`: required when `type` is `currency`

Ability command grouping:

- `command_group`: optional string used to route abilities to `abilities_group` commands.
- `ability_id` commands bypass grouping and should omit `command_group` on the target ability.

Targeting fields:

- `target_mode`: `single`, `multi`, or `both`.
- `multi_attenuation`: optional multiplier applied per target when in multi mode.

```json
{
  "version": 1,
  "abilities": [
    {
      "id": "power_strike",
      "name": "Power Strike",
      "description": "A heavy blow that hits harder than a basic attack.",
      "default_target": "enemy",
      "allowed_targets": ["enemy"],
      "target_mode": "single",
      "multi_attenuation": 0.6,
      "effect": {"type": "damage", "power": 6},
      "cost": {"type": "hp", "value": 4}
    }
  ]
}
```

## entities/items.json

Item usage defines where and how items can be used. `context` values: `field`,
`battle`, or `both`. `target` values: `self`, `ally`, `party`, `enemy`.

`description` is optional text displayed in item menus.
Common effect types: `heal_hp`, `heal_mp`, `revive`, `warp`, `learn_spell`, `learn_recipe`, `cure_status`.
`learn_spell` uses `effect.target` to specify the spell ID to teach.
`learn_recipe` uses `effect.target` to specify the recipe ID to unlock.
`cure_status` uses `effect.statuses` to list status IDs to remove.
`warp` supports `effect.target: "last_overworld"` to return to the last overworld entry and
`effect.destination` to warp to a specific map/position.

```json
{
  "version": 1,
  "items": [
    {
      "id": "potion",
      "name": "Potion",
      "type": "consumable",
      "description": "Restores a small amount of HP.",
      "usage": {"context": "field", "target": "ally"},
      "effect": {"type": "heal_hp", "power": 50}
    },
    {
      "id": "antidote",
      "name": "Antidote",
      "type": "consumable",
      "usage": {"context": "both", "target": "ally"},
      "effect": {"type": "cure_status", "statuses": ["poison"]}
    },
    {
      "id": "warp_scroll",
      "name": "Warp Scroll",
      "type": "travel",
      "usage": {"context": "field", "target": "party"},
      "effect": {
        "type": "warp",
        "target": "last_overworld"
      }
    },
    {
      "id": "tome_fire",
      "name": "Tome of Fire",
      "type": "consumable",
      "usage": {"context": "field", "target": "ally"},
      "effect": {"type": "learn_spell", "target": "fire"}
    }
  ]
}
```

## entities/shops.json

```json
{
  "version": 1,
  "shops": [
    {
      "id": "corner_store",
      "name": "Corner Store",
      "currency": "gold",
      "inventory": [
        {"item": "potion", "price": 50},
        {"item": "bronze_sword", "price": 200}
      ]
    }
  ]
}
```

## entities/equipment.json

Equipment categories should align with job equipment lists (e.g., job weapons list
contains categories like "sword", "staff", while `slot` describes where it equips).
Use `allowed_jobs` only for item-specific overrides (e.g., a katana requiring Samurai).
`spells` (optional) lists spell IDs granted while the equipment is equipped. For
Magic Equip items, use `slot: "magic"` and include the spells they should grant.

```json
{
  "version": 1,
  "equipment": [
    {
      "id": "bronze_sword",
      "name": "Bronze Sword",
      "category": "sword",
      "slot": "weapon",
      "allowed_jobs": null,
      "stats": {"str": 2},
      "spells": ["fire"]
    }
  ]
}

Equipment spells:

- `spells`: list of spell IDs provided by the equipment while equipped.
```

## entities/enemies.json

```json
{
  "version": 1,
  "enemies": [
    {
      "id": "imp",
      "name": "Imp",
      "stats": {"hp": 12, "mp": 0, "str": 3, "int": 1, "agi": 1},
      "traits": ["beast"],
      "sprite": {
        "glyph": "i",
        "palette": "enemy"
      },
      "art": {
        "lines": [
          " /\\ ",
          "( ..)",
          " /__\\"
        ],
        "palette": "enemy"
      },
      "exp": 6,
      "currency": [{"id": "gold", "amount": 8}],
      "loot": [{"item": "potion", "chance": 0.1}]
    }
  ]
}
```

Traits are used by effects (e.g., `undead` can invert healing).

`exp` and `currency` stacks are rewarded per enemy and summed at victory.

## entities/vehicles.json

```json
{
  "version": 1,
  "vehicles": [
    {
      "id": "ship",
      "name": "Ship",
      "speed": 2,
      "allowed_tiles": ["water"],
      "unlock_flag": "vehicle.ship_unlocked",
      "glyph": "S",
      "palette": "bright_cyan"
    }
  ]
}
```

Vehicle fields:

- `glyph`: optional single-character glyph used for overworld rendering (defaults to `V`).
- `palette`: optional palette name for the vehicle glyph.

## events/*.json

Event scripts are ordered lists of actions.

```json
{
  "version": 1,
  "id": "intro_scene",
  "steps": [
    {"type": "dialog", "speaker": "Elder", "text": "The crystals are fading..."},
    {"type": "narration", "text": "A chill wind sweeps the valley."},
    {"type": "require_flags", "flags": ["dungeon.ember_cleared", "dungeon.tide_cleared"]},
    {"type": "set_flag", "flag": "world.intro_complete"},
    {"type": "give_item", "item": "potion", "qty": 1}
  ]
}
```

Supported event step types:

- `dialog` (fields: `speaker`, `text`)
- `narration` (fields: `text`)
- `set_flag` (fields: `flag`)
- `require_flags` (fields: `flags` list)
- `give_item` (fields: `item`, `qty`)
- `give_equipment` (fields: `item`, `qty`)
- `learn_spell` (fields: `member`, `spell`)
- `party_add` (fields: `member`)
- `party_remove` (fields: `member`)
- `learn_recipe` (fields: `recipe`)
- `warp` (fields: `target` with `map` and `pos`; use `map: "last_overworld"` to return to the last overworld entry)
- `start_battle` (fields: `encounter`, `formation`)
- `open_shop` (fields: `shop`)
- `npc_show`, `npc_hide`, `npc_move`, `npc_set_sprite` (fields: `npc`, `pos`, `sprite`)
- `start_dialog` (fields: `dialog`)
- `wait` (fields: `ms`)
- `stat_set` (fields: `stat`, `value`)
- `stat_add` (fields: `stat`, `value`, default 1)
- `stat_max` (fields: `stat`, `value`)

`party_add` pulls the member from `party.json` roster and places them in the active party if there is space, otherwise in reserve. `party_remove` only removes the member from active/reserve lists; the roster entry remains so the member can rejoin later. Both steps abort the event if the member cannot be added/removed.

`npc_show` applies `pos` when provided; `npc_move` requires `pos` to update the NPC location.

Mode notes:
- `preset` and `preset_rename` always use `party.json`, so `party_add` is available.
- `create` only supports `party_add` when `party.json` is present; if missing, the event will abort.
- `party_add` does not trigger rename prompts; rename-on-join is planned separately.

Event trigger types (for `maps/*/json` `events` entries):

- `on_enter`: Fires when entering a map.
- `on_step`: Fires when stepping on a tile or crossing a zone boundary.
  - For coordinate-based: Include `pos` field with `[x, y]` coordinates.
  - For zone-based: Include `zone` field matching an `encounters` zone ID.

Example steps:

```json
{
  "type": "give_equipment",
  "item": "bronze_sword",
  "qty": 1
}
```

```json
{
  "type": "learn_spell",
  "member": "alric",
  "spell": "fire"
}
```

```json
{
  "type": "party_add",
  "member": "alric"
}
```

```json
{
  "type": "party_remove",
  "member": "alric"
}
```

```json
{
  "type": "warp",
  "target": {"map": "overworld_gaia", "pos": [6, 4]}
}
```

```json
{
  "type": "start_battle",
  "encounter": "gaia_grasslands",
  "formation": [
    {"enemy": "imp", "pos": [0, 1]},
    {"enemy": "wisp", "pos": [2, 1]}
  ]
}
```

```json
{
  "type": "start_dialog",
  "dialog": "sky_tinker"
}
```

```json
{
  "type": "stat_add",
  "stat": "crystals_collected",
  "value": 1
}
```

## entities/quests.json

Quest definitions for the Journal system. Quests are organized by categories with
configurable sort order. Each quest has a title, category reference, and a tree of
steps with optional substeps. Quest progress is tracked via flags using the
`quest.<quest_id>.<step_id>` naming convention.

```json
{
  "version": 1,
  "categories": [
    {
      "id": "main",
      "label": "Main Quests",
      "sort_order": 10
    },
    {
      "id": "side",
      "label": "Side Quests",
      "sort_order": 20
    }
  ],
  "quests": [
    {
      "id": "tide_shards",
      "title": "Shards of the Tide",
      "category_id": "main",
      "steps": [
        {
          "id": "started",
          "text": "Speak with the Harbor Historian about the Tide Crystal shards.",
          "flag": "quest.tide_shards.started",
          "substeps": []
        },
        {
          "id": "find_shards",
          "text": "Find the Tide shards in the harbor caves.",
          "flag": "quest.tide_shards.find_shards",
          "substeps": [
            {
              "id": "cave_1",
              "text": "Search the northern cave.",
              "flag": "quest.tide_shards.cave_1"
            },
            {
              "id": "cave_2",
              "text": "Search the southern cave.",
              "flag": "quest.tide_shards.cave_2"
            }
          ]
        },
        {
          "id": "return_shards",
          "text": "Return the shards to the Harbor Historian.",
          "flag": "quest.tide_shards.return_shards",
          "substeps": []
        }
      ]
    },
    {
      "id": "lost_cat",
      "title": "The Lost Cat",
      "category_id": "side",
      "steps": [
        {
          "id": "find_cat",
          "text": "Find the missing cat in the alley.",
          "flag": "quest.lost_cat.find_cat",
          "substeps": []
        },
        {
          "id": "return_cat",
          "text": "Return the cat to the innkeeper.",
          "flag": "quest.lost_cat.return_cat",
          "substeps": []
        }
      ]
    }
  ]
}
```

Quest category fields:

- `id`: unique category identifier (lowercase snake_case).
- `label`: display name shown in the Journal UI.
- `sort_order`: numeric order for category display (lower values appear first).

Quest fields:

- `id`: unique quest identifier (lowercase snake_case).
- `title`: display name shown in the Journal UI.
- `category_id`: references a category `id` from the `categories` list.
- `steps`: ordered list of quest steps.

Quest step fields:

- `id`: unique step identifier within the quest (lowercase snake_case).
- `text`: description shown in the Journal UI.
- `flag`: flag name that tracks step completion (format: `quest.<quest_id>.<step_id>`).
- `show_flag`: optional flag name that reveals the step before it is complete.
- `substeps`: optional nested steps for multi-part objectives.

Flag naming conventions:

- Quest flags use the format `quest.<quest_id>.<step_id>` for step completion.
- Map/environmental flags use `map.<map_id>.<event_id>` (e.g., `map.town.bridge_repaired`).
- System flags use `system.<feature_id>` (e.g., `system.fast_travel_unlocked`).
- Vehicle flags use `vehicle.<vehicle_id>` (e.g., `vehicle.airship_unlocked`).
- Dungeon flags use `dungeon.<dungeon_id>` (e.g., `dungeon.ember_cleared`).

Quest visibility and completion rules:

- A quest becomes visible only when its first step flag is set (quest acquired).
- A step is complete when its `flag` is set.
- A completed step is always visible once the quest is acquired.
- If `show_flag` is set, incomplete steps remain hidden until that flag is set.
- If `show_flag` is omitted, an incomplete step is visible only after the previous step in the same list is complete.
- Substeps follow the same visibility/completion rules as parent steps.
- History entries are derived from completed steps in the order they appear in the quest definition.

## ui/gameplay_stats.json

Configurable menu panels for gameplay stats.

```json
{
  "version": 1,
  "panels": [
    {
      "id": "crystal_progress",
      "title": "Crystals",
      "items": [
        {"label": "Crystals", "value": "crystals_collected", "max": 4},
        {"label": "Dungeons", "value": "dungeons_cleared"}
      ]
    }
  ]
}
```

## ui/menu.json

Defines the main menu layout and entry list. The menu is a two-pane layout with a
left list and right detail pane. The right pane defaults to party/status summary
until a submenu is confirmed.

Menu entries can be gated by a rules `systems` toggle and an optional
`unlock_flag`. If gating fails, the entry is hidden by default; set
`locked_behavior` to `disable` to show it disabled instead.

```json
{
  "version": 1,
  "layout": {
    "left_width_ratio": 0.4,
    "right_width_ratio": 0.6
  },
  "default_panel": "party_status",
  "menu": [
    {
      "id": "items",
      "label": "Items",
      "action": "items",
      "system": "items"
    },
    {
      "id": "magic",
      "label": "Magic",
      "action": "magic",
      "system": "magic",
      "locked_behavior": "disable"
    },
    {
      "id": "summons",
      "label": "Summons",
      "action": "summons",
      "system": "summons",
      "unlock_flag": "system.summons_unlocked",
      "locked_behavior": "disable"
    },
    {
      "id": "journal",
      "label": "Journal",
      "action": "journal",
      "system": "journal"
    },
    {
      "id": "save",
      "label": "Save",
      "action": "save",
      "system": "save",
      "unlock_flag": "system.save_unlocked",
      "locked_behavior": "disable"
    },
    {
      "id": "settings",
      "label": "Settings",
      "action": "settings",
      "system": "settings"
    }
  ],
  "panels": [
    {
      "id": "party_status",
      "title": "Party",
      "type": "party_summary"
    },
    {
      "id": "gameplay_stats",
      "title": "Gameplay Stats",
      "type": "progress",
      "source": "ui/gameplay_stats.json"
    }
  ]
}
```

Menu entry fields:

- `action`: built-in submenu or command identifier.
- `enabled`: optional boolean to disable the entry entirely.
- `system`: rules `systems` key that must be true.
- `unlock_flag`: optional flag required to unlock the entry.
- `locked_behavior`: `hide` (default) or `disable`.

Runtime notes:

- Save is also gated by map `allow_save` and `save_points`; the menu can display
  a disabled entry even if the save system exists.
- The Overworld Map menu panel is view-only until fast travel is unlocked via the
  world `fast_travel` config and the `systems.fast_travel` rule toggle.

## ui/title.json

Defines the title screen layout and content. This is a lightweight, templatable
configuration intended to support ASCII logos, attribution, and menu items.
The optional `gameover` block configures the gameover screen menu.

Logo fields:

- `lines`: array of ASCII logo rows.
- `palette`: optional palette name applied to every logo row.
- `line_palettes`: optional array of palette names applied per row (falls back to
  `palette` if an entry is missing).

Gameover fields:

- `title`: optional heading override (defaults to `Game Over`).
- `subtitle`: optional subheading string.
- `menu`: list of menu items (ids include `retry_battle`, `load_latest`, `load_autosave`,
  `return_title`, `exit`).
- `footer`: optional footer override (falls back to the title footer).

Localization notes:

- Gameover text can be localized via `ui/strings.json` using keys like
  `gameover.title`, `gameover.subtitle`, `gameover.retry_battle`, `gameover.load_latest`,
  `gameover.load_autosave`, `gameover.return_title`, and `gameover.exit`.
- When a localization key is missing, the `ui/title.json` label is used as the fallback.

```json
{
  "version": 1,
  "title": "OpenCrystal",
  "logo": {
    "lines": [
      "    .   .   /#\\     .",
      "  *        /*##\\  .    *",
      "'      .  /**###\\    .  '",
      "    <    /***####\\      >",
      "  .   *  \\***####/  *   .",
      "*       . \\**###/   .    *",
      "    .      \\*##/  *      .",
      "  '      *  \\#/  .    '",
      "     OpenCrystal Engine"
    ],
    "line_palettes": [
      "bright_cyan",
      "bright_cyan",
      "bright_cyan",
      "bright_cyan",
      "cyan",
      "cyan",
      "cyan",
      "cyan",
      "white"
    ]
  },
  "menu": [
    {"id": "new_game", "label": "New Game"},
    {"id": "load_game", "label": "Load"},
    {"id": "settings", "label": "Settings"},
    {"id": "exit", "label": "Exit"}
  ],
  "gameover": {
    "title": "Game Over",
    "subtitle": "The party has fallen.",
    "menu": [
      {"id": "retry_battle", "label": "Retry Battle"},
      {"id": "load_latest", "label": "Load Latest Save"},
      {"id": "load_autosave", "label": "Load Autosave"},
      {"id": "return_title", "label": "Return to Title"},
      {"id": "exit", "label": "Exit"}
    ],
    "footer": {
      "left": "A crystal-bound journey",
      "right": "By OpenCrystal Team"
    }
  },
  "footer": {
    "left": "A crystal-bound journey",
    "right": "By OpenCrystal Team"
  }
}
```

## ui/strings.json

Localized UI strings keyed by id. The engine falls back to built-in defaults when a key is
missing, so this file is optional but recommended.

```json
{
  "version": 1,
  "strings": {
    "command.attack": "Attack",
    "battle.victory": "Victory!",
    "battle.no_targets": "No valid targets."
  }
}
```

## ui/battle.json

Defines battle UI layout and panel behavior. Panels can be anchored or flexed to adapt
to terminal sizes. `party_grid.columns` controls how player sprites are arranged in the
battlefield grid (default 1 for a vertical line).

Battle log positions:

- `top`: top of the screen.
- `pane_top`: single row above the command row panels.

Battle dialog positions:

- `top`: overlays the top of the screen.
- `bottom`: overlays the bottom of the screen.

Battle dialog timing fields:

- `auto_advance_ms`: delay before auto-advance (0 disables auto-advance).
- `allow_skip`: allow Confirm/Cancel to skip wait.

Battle animation fields:

- `flash_ms`: delay per flash frame.
- `flash_cycles`: number of flash cycles per action.
- `panels.commands.page_size`: number of command entries per page (adds page indicator when needed).

```json
{
  "version": 1,
  "breakpoints": [
    {
      "id": "compact",
      "min_width": 0,
      "min_height": 0,
      "behavior": {
        "enemy_art": "glyph",
        "hide_panel_titles": true
      }
    },
    {
      "id": "standard",
      "min_width": 110,
      "min_height": 32,
      "behavior": {
        "enemy_art": "auto",
        "hide_panel_titles": false
      }
    }
  ],
  "layout": {
    "battlefield": {"anchor": "top", "height_ratio": 0.6},
    "command_row": {
      "anchor": "bottom",
      "height_ratio": 0.4,
      "columns": [
        {"id": "enemies", "width_ratio": 0.3},
        {"id": "commands", "width_ratio": 0.3},
        {"id": "party", "width_ratio": 0.4}
      ]
    },
    "party_grid": {
      "columns": 1
    }
  },
  "log": {
    "position": "pane_top",
    "height": 3,
    "auto_advance_ms": 700,
    "allow_skip": true
  },
  "animation": {
    "flash_ms": 150,
    "flash_cycles": 2
  },
  "panels": {
    "enemies": {
      "title": "Enemies",
      "highlight": {"style": "invert", "link_to_battlefield": true}
    },
    "commands": {
      "title": "Commands",
      "items": ["Attack", "Magic", "Abilities", "Items", "Defend", "Run"],
      "page_size": 6
    },
    "party": {
      "title": "Party",
      "show": ["hp", "mp", "readiness", "status"],
      "highlight": {"style": "underline", "link_to_battlefield": true}
    }
  },
  "menus": {
    "attack": {"target": "enemy"},
    "magic": {
      "list": "spells",
      "group_by": "school",
      "columns": [
        {"id": "name", "label": "Spell"},
        {"id": "cost", "label": "MP"}
      ],
      "target_from_spell": true
    },
    "abilities": {
      "list": "abilities",
      "columns": [
        {"id": "name", "label": "Ability"}
      ],
      "target_from_ability": true
    },
    "items": {
      "list": "inventory",
      "columns": [
        {"id": "name", "label": "Item"},
        {"id": "qty", "label": "Qty"}
      ],
      "target_from_item": true
    }
  },
  "selection": {
    "target_cursor": "blink",
    "battlefield_highlight": "outline",
    "list_highlight": "invert"
  }
}
```

## ui/dialog.json

Defines the dialog box layout and behavior. Dialog text is auto-wrapped into multiple
pages based on available width and height.

```json
{
  "version": 1,
  "position": "bottom",
  "height": 4,
  "show_speaker": true,
  "continue_marker": "▼"
}
```

## stats.json

Defines base stats and derived stats. This allows games to extend or rename stats.

Formula expressions are strings. Supported operators: `+ - * /` and parentheses. Supported
functions: `RAND(min,max)` (inclusive, float), `ROUND(value)` (nearest int), `FLOOR(value)`,
`CEIL(value)`. Formulas may reference base stats, `lvl`, and `gear.*`/`buffs.*` for derived stats.

```json
{
  "version": 1,
  "stats": {
    "base": [
      {"id": "hp", "name": "HP", "min": 0},
      {"id": "mp", "name": "MP", "min": 0},
      {"id": "str", "name": "STR"},
      {"id": "int", "name": "INT"},
      {"id": "vit", "name": "VIT"},
      {"id": "agi", "name": "AGI"},
      {"id": "lck", "name": "LCK"}
    ],
    "derived": [
      {"id": "atk", "name": "ATK"},
      {"id": "matk", "name": "MATK"},
      {"id": "def", "name": "DEF"},
      {"id": "mdef", "name": "MDEF"},
      {"id": "eva", "name": "EVA"}
    ],
    "formulas": {
      "atk": "str * 2 + gear.atk + buffs.atk",
      "matk": "int * 2 + gear.matk + buffs.matk",
      "def": "vit * 2 + gear.def + buffs.def",
      "mdef": "int + vit + gear.mdef + buffs.mdef",
      "eva": "agi + lck / 2 + gear.eva + buffs.eva"
    }
  }
}
```

## save.json

Save data captures the full runtime state. This format is stored as JSON in phase 1.
`timestamp_seconds` is a UNIX epoch timestamp (UTC) for the save creation time.

`settings` stores user-configurable settings captured at save time. Rules overrides
are still applied when loading.

Vehicle fields:

- `world.vehicle`: active vehicle id or null when on foot.
- `vehicles`: per-vehicle map positions keyed by vehicle id.
- `party.members.*.row`: `front` or `back` when battle rows are enabled.

```json
{
  "version": 1,
  "encoding": "plain",
  "metadata": {
    "slot": 1,
    "title": "OpenCrystal",
    "play_time_seconds": 3210,
    "timestamp_seconds": 1769085720
  },
  "rules": {
    "battle_mode": "turn",
    "magic_system": "mp"
  },
  "world": {
    "world_id": "gaia",
    "map_id": "overworld_gaia",
    "pos": [20, 14],
    "vehicle": "ship"
  },
  "vehicles": {
    "ship": {"map_id": "overworld_gaia", "pos": [0, 1]},
    "airship": {"map_id": "overworld_gaia", "pos": [8, 8]}
  },
  "map_state": {
    "overworld_gaia": {
      "flags": {
        "chest_1_opened": true,
        "bridge_repaired": false
      },
      "entities": {
        "wandering_merchant": {"pos": [12, 9], "state": "wandering"},
        "roaming_behemoth": {"pos": [44, 22], "state": "asleep"}
      }
    }
  },
  "global_entities": {
    "airship_01": {
      "type": "vehicle",
      "entity_id": "airship",
      "world": "gaia",
      "map": "overworld_gaia",
      "pos": [30, 18],
      "state": "idle"
    }
  },
  "party": {
    "active": ["hero_1", "hero_2", "hero_3", "hero_4"],
    "reserve": ["hero_5"],
    "members": {
      "hero_1": {
        "name": "Luna",
        "job": "white_mage",
        "row": "front",
        "level": 3,
        "exp": 120,
        "stats": {"hp": 28, "mp": 12, "str": 2, "int": 8, "vit": 4, "agi": 5, "lck": 3},
        "equipment": {"weapon": "bronze_staff", "armor": "linen_robe"},
        "spells": ["cure"],
        "status": []
      }
    }
  },
  "inventory": {
    "currency": {"gold": 250},
    "items": [{"id": "potion", "qty": 3}],
    "equipment": [{"id": "bronze_sword", "qty": 1}]
  },
  "settings": {
    "autosave_enabled": true,
    "readiness_speed": 2.5,
    "battle_mode": "dynamic_wait"
  },
  "flags": {
    "world": {
      "intro_complete": true,
      "job_change_unlocked": false
    },
    "quests": {
      "crystal_fire": "in_progress",
      "tide_shards": "started"
    }
  },
  "stats": {
    "time_played": 3210,
    "enemies_defeated": 14,
    "max_damage": 32,
    "jobs_unlocked": 4,
    "crystals_collected": 1
  },
  "journal": {
    "entries": [
      {
        "id": "quest_fire",
        "title": "The Fire Crystal",
        "status": "in_progress",
        "steps": [
          {
            "id": "reach_ember",
            "text": "Reach the Ember Caverns.",
            "visible": true,
            "complete": true
          },
          {
            "id": "restore_fire",
            "text": "Restore the Fire Crystal.",
            "visible": true,
            "complete": false
          }
        ]
      },
      {
        "id": "quest_tide_shards",
        "title": "Shards of the Tide",
        "status": "not_started",
        "steps": [
          {
            "id": "find_shards",
            "text": "Find the Tide shards in the harbor caves.",
            "visible": false,
            "complete": false
          },
          {
            "id": "return_shards",
            "text": "Return the shards to the Harbor Historian.",
            "visible": false,
            "complete": false
          }
        ]
      }
    ]
  }
}
```

Quest tracking conventions:

- `status` is one of `not_started`, `in_progress`, `complete`.
- `steps` is an ordered list of quest milestones.
- `visible` controls whether a step shows in the journal UI.
- `complete` tracks step completion.
- Events/dialog actions should update quest `status`, set `visible`, and mark `complete`.

Example quest updates (dialog actions):

```json
{
  "type": "set_flag",
  "flag": "quest.tide_shards.started"
}
```

```json
{
  "type": "set_flag",
  "flag": "quest.tide_shards.find_shards"
}
```

Example quest updates (event steps):

```json
{
  "type": "set_flag",
  "flag": "quest.tide_shards.return_shards"
}
```

Flags are mapped to quest steps by game rules or a quest resolver.

## Battle mock bundle (example)

This minimal bundle exercises the battle UI and selection flow. These snippets correspond
to the schemas defined above and consolidate the former `BATTLE_MOCK.md` examples.

### entities/enemies.json

```json
{
  "version": 1,
  "enemies": [
    {
      "id": "imp",
      "name": "Imp",
      "stats": {"hp": 18, "mp": 0, "str": 3, "int": 1},
      "traits": ["beast"],
      "sprite": {"glyph": "i", "palette": "enemy"},
      "art": {"lines": [" /\\ ", "( ..)", " /__\\"], "palette": "enemy"},
      "exp": 6,
      "currency": [{"id": "gold", "amount": 8}],
      "loot": [{"item": "potion", "chance": 0.2}]
    },
    {
      "id": "wisp",
      "name": "Wisp",
      "stats": {"hp": 14, "mp": 8, "str": 2, "int": 4},
      "traits": ["undead"],
      "sprite": {"glyph": "*", "palette": "enemy_magic"},
      "art": {"lines": [" .*. ", "( * )", " ' ' "], "palette": "enemy_magic"},
      "exp": 8,
      "currency": [{"id": "gold", "amount": 12}],
      "loot": [{"item": "ether", "chance": 0.1}]
    }
  ]
}
```

### entities/encounters.json

```json
{
  "version": 1,
  "tables": [
    {
      "id": "gaia_grasslands",
      "entries": [
        {
          "weight": 60,
          "formation": [
            {"enemy": "imp", "pos": [0, 1]},
            {"enemy": "imp", "pos": [1, 2]},
            {"enemy": "imp", "pos": [2, 3]}
          ]
        },
        {
          "weight": 40,
          "formation": [
            {"enemy": "wisp", "pos": [1, 2]},
            {"enemy": "wisp", "pos": [2, 3]}
          ]
        },
        {
          "weight": 20,
          "formation": [
            {"enemy": "imp", "pos": [0, 1]},
            {"enemy": "imp", "pos": [1, 2]},
            {"enemy": "wisp", "pos": [3, 0]}
          ]
        }
      ]
    }
  ]
}
```

### entities/spells.json

```json
{
  "version": 1,
  "schools": [
    {"id": "white", "name": "White"},
    {"id": "black", "name": "Black"}
  ],
  "spells": [
    {
      "id": "cure",
      "name": "Cure",
      "school": "white",
      "tier": 1,
      "cost": {"type": "mp", "value": 3},
      "default_target": "ally",
      "allowed_targets": ["ally", "enemy"],
      "effect": {"type": "heal", "power": 20}
    },
    {
      "id": "fire",
      "name": "Fire",
      "school": "black",
      "tier": 1,
      "cost": {"type": "mp", "value": 4},
      "default_target": "enemy",
      "allowed_targets": ["enemy"],
      "effect": {"type": "damage", "element": "fire", "power": 22}
    }
  ]
}
```

### rules.json

```json
{
  "version": 1,
  "game": {
    "title": "OpenCrystal",
    "party_size": 4,
    "party_reserve_size": 4,
    "battle_mode": "turn",
    "magic_system": "mp",
    "currencies": [{"id": "gold", "name": "Gold", "symbol": "G"}]
  },
  "party_mode": "create",
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
  "render": {
    "min_art_width": 110,
    "min_art_height": 32,
    "palette": "terminal"
  }
}
```

### entities/jobs.json

```json
{
  "version": 1,
  "jobs": [
    {
      "id": "fighter",
      "name": "Fighter",
      "stats": {"hp": 30, "mp": 0, "str": 8, "int": 2},
      "growth": {
        "type": "formula",
        "per_level": {
          "hp": "6 + vit",
          "mp": "0",
          "str": "2",
          "vit": "2",
          "agi": "1",
          "int": "0",
          "lck": "1"
        }
      },
      "equipment": {"weapons": ["sword"], "armor": ["plate"]},
      "spells": []
    },
    {
      "id": "white_mage",
      "name": "White Mage",
      "stats": {"hp": 20, "mp": 10, "str": 2, "int": 8},
      "growth": {
        "type": "formula",
        "per_level": {
          "hp": "3 + ROUND(RAND(-1,1))",
          "mp": "2 + ROUND(RAND(0,1) * 3)",
          "str": "0",
          "vit": "1",
          "agi": "1",
          "int": "2",
          "lck": "1"
        }
      },
      "equipment": {"weapons": ["staff"], "armor": ["robe"]},
      "spells": ["cure"]
    }
  ]
}
```

### ui/battle.json

```json
{
  "version": 1,
  "breakpoints": [
    {
      "id": "compact",
      "min_width": 0,
      "min_height": 0,
      "behavior": {
        "enemy_art": "glyph",
        "hide_panel_titles": true
      }
    },
    {
      "id": "standard",
      "min_width": 110,
      "min_height": 32,
      "behavior": {
        "enemy_art": "auto",
        "hide_panel_titles": false
      }
    }
  ],
  "layout": {
    "battlefield": {"anchor": "top", "height_ratio": 0.6},
    "command_row": {
      "anchor": "bottom",
      "height_ratio": 0.4,
      "columns": [
        {"id": "enemies", "width_ratio": 0.3},
        {"id": "commands", "width_ratio": 0.4},
        {"id": "party", "width_ratio": 0.3}
      ]
    }
  },
  "log": {"position": "pane_top", "height": 1},
  "panels": {
    "enemies": {"title": "Enemies", "highlight": {"style": "invert", "link_to_battlefield": true}},
    "commands": {"title": "Commands", "items": ["Attack", "Magic", "Items", "Run"]},
    "party": {"title": "Party", "show": ["hp", "mp", "readiness", "status"], "highlight": {"style": "underline", "link_to_battlefield": true}}
  },
  "menus": {
    "attack": {"target": "enemy"},
    "magic": {
      "list": "spells",
      "group_by": "school",
      "columns": [
        {"id": "name", "label": "Spell"},
        {"id": "cost", "label": "MP"}
      ],
      "target_from_spell": true
    },
    "items": {
      "list": "inventory",
      "columns": [
        {"id": "name", "label": "Item"},
        {"id": "qty", "label": "Qty"}
      ],
      "target_from_item": true
    }
  },
  "selection": {
    "target_cursor": "blink",
    "battlefield_highlight": "outline",
    "list_highlight": "invert"
  }
}
```
