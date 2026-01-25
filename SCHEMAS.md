# OpenCrystal JSON Schemas (Draft)

This document defines the core JSON formats for OpenCrystal content. These are draft schemas
intended to guide implementation and are subject to change as the engine matures.

All files are UTF-8 JSON. Each top-level schema includes a `version` field for forward
compatibility.

## Shared conventions

- All IDs are lowercase snake_case strings.
- All entity references use IDs, not filenames.
- Missing optional fields should fall back to documented defaults.
- Palette values use terminal color names (`red`, `green`, `blue`, etc.) plus bright variants
  (`bright_red`, `bright_green`, etc.). These map to the user's terminal theme.

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

## rules.json

Global rules and feature flags.

`systems` toggles whether a gameplay system/menu is enabled at all. It should be
used for global availability (e.g., disabling materia). Menu entries can still
add `unlock_flag` gating for progression-driven unlocks.

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
    "start_event": "intro_cutscene",
    "job_change_enabled": false,
    "job_change_flag": "world.job_change_unlocked",
    "currency": {"id": "gil", "name": "G", "symbol": "G"}
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
  "features": {
    "journal": true,
    "fast_travel": true,
    "overworld_map": true
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
      "crystals_collected"
    ]
  }
}
```

## worlds.json

Defines multiple worlds and inter-world travel.

```json
{
  "version": 1,
  "worlds": [
    {
      "id": "gaia",
      "name": "Gaia",
      "starting_map": "overworld_gaia",
      "zoom_levels": ["overview", "explore"],
      "overview": {
        "enabled": true,
        "map_id": "gaia_overview"
      },
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
uses its dialog tree.

`hide_name` controls whether the map name tooltip is shown on entry. Defaults to false.

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
  "events": [
    {
      "id": "intro",
      "trigger": "on_enter",
      "script": "intro_scene"
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
  "shops": [
    {"id": "corner_store", "pos": [8, 6]}
  ],
  "allow_save": true,
  "save_points": [[12, 9]],
  "transitions": [
    {
      "id": "to_ember",
      "pos": [5, 8],
      "target_map": "dungeon_ember",
      "target_pos": [1, 1],
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

Rendering notes:

- `legend.palette` sets the terminal palette name for the tile glyph.
- `transitions` may include a `glyph` override and `palette` to highlight exits.

Sign notes:

- `signs.glyph` is optional; defaults to `⚑` if omitted.
- `signs.text` appears in a centered dialog when the player confirms nearby.
- Signs block movement and are not passable.
- Signs are interactive objects defined directly on maps; they are not NPCs.

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
      "behavior": {"type": "roam", "radius": 4, "persist": true}
    }
  ]
}
```

Behavior types (initial): `static`, `roam`, `patrol`. For `patrol`, use `behavior.path`.
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

## entities/jobs.json

Growth formulas can reference current base stats (e.g., `vit`, `int`) as inputs.
Growth expressions follow the same syntax as `stats.json` formulas.

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
      "spells": ["cure", "protect"]
    }
  ]
}
```

## entities/spells.json

Supports multiple magic schools.

Spell costs are flexible. Use `{"type": "mp"}` for MP-based casting or
`{"type": "tier_charges"}` for limited-per-tier systems.

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
    }
  ]
}
```

## entities/items.json

```json
{
  "version": 1,
  "items": [
    {
      "id": "potion",
      "name": "Potion",
      "type": "consumable",
      "effect": {"type": "heal", "power": 50}
    },
    {
      "id": "warp_scroll",
      "name": "Warp Scroll",
      "type": "travel",
      "effect": {
        "type": "warp",
        "target": "world_map",
        "destination": {"map": "overworld_gaia", "pos": [20, 14]}
      }
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
      "currency": "gil",
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
      "stats": {"str": 2}
    }
  ]
}
```

## entities/enemies.json

```json
{
  "version": 1,
  "enemies": [
    {
      "id": "imp",
      "name": "Imp",
      "stats": {"hp": 12, "mp": 0, "str": 3, "int": 1},
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
      "loot": [{"item": "potion", "chance": 0.1}]
    }
  ]
}
```

Traits are used by effects (e.g., `undead` can invert healing).

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
      "unlock_flag": "vehicle.ship_unlocked"
    }
  ]
}
```

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

- `dialog`
- `narration`
- `set_flag`
- `require_flags`
- `give_item`
- `give_equipment`
- `warp`
- `start_battle`
- `open_shop`
- `npc_show`
- `npc_hide`
- `npc_move`
- `npc_set_sprite`
- `start_dialog`

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

## ui/progress.json

Configurable menu panels for progress stats.

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
      "id": "progress",
      "title": "Progress",
      "type": "progress",
      "source": "ui/progress.json"
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

## ui/title.json

Defines the title screen layout and content. This is a lightweight, templatable
configuration intended to support ASCII logos, attribution, and menu items.

```json
{
  "version": 1,
  "title": "OpenCrystal",
  "logo": {
    "lines": [
      "  ___                     ____          _       _ ",
      " / _ \\ _ __   ___ _ __   / ___|_ __ ___| |_ __ _| |",
      "| | | | '_ \\ / _ \\ '_ \\ | |   | '__/ __| __/ _` | |",
      "| |_| | |_) |  __/ | | || |___| | | (__| || (_| | |",
      " \\___/| .__/ \\___|_| |_| \\____|_|  \\___|\\__\\__,_|_|",
      "      |_|                                            "
    ]
  },
  "menu": [
    {"id": "new_game", "label": "New Game"},
    {"id": "load_game", "label": "Load"},
    {"id": "settings", "label": "Settings"},
    {"id": "exit", "label": "Exit"}
  ],
  "footer": {
    "left": "A crystal-bound journey",
    "right": "By OpenCrystal Team"
  }
}
```

## ui/battle.json

Defines battle UI layout and panel behavior. Panels can be anchored or flexed to adapt
to terminal sizes.

Battle log positions:

- `top`: top of the screen.
- `pane_top`: single row above the command row panels.

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
  "log": {
    "position": "top",
    "height": 2
  },
  "panels": {
    "enemies": {
      "title": "Enemies",
      "highlight": {"style": "invert", "link_to_battlefield": true}
    },
    "commands": {
      "title": "Commands",
      "items": ["Attack", "Magic", "Items", "Run"]
    },
    "party": {
      "title": "Party",
      "show": ["hp", "mp", "atb", "status"],
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
`CEIL(value)`. Formulas may reference base stats and `gear.*`/`buffs.*` for derived stats.

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

```json
{
  "version": 1,
  "encoding": "plain",
  "metadata": {
    "slot": 1,
    "title": "OpenCrystal",
    "play_time_seconds": 3210,
    "timestamp": "2026-01-22T10:02:00Z"
  },
  "rules": {
    "battle_mode": "turn",
    "magic_system": "mp"
  },
  "world": {
    "world_id": "gaia",
    "map_id": "overworld_gaia",
    "pos": [20, 14],
    "vehicle": null
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
    "currency": {"id": "gil", "amount": 250},
    "items": [{"id": "potion", "qty": 3}],
    "equipment": [{"id": "bronze_sword", "qty": 1}]
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
