# OpenCrystal JSON Schemas (Draft)

This document defines the core JSON formats for OpenCrystal content. These are draft schemas
intended to guide implementation and are subject to change as the engine matures.

All files are UTF-8 JSON. Each top-level schema includes a `version` field for forward
compatibility.

## Shared conventions

- All IDs are lowercase snake_case strings.
- All entity references use IDs, not filenames.
- Missing optional fields should fall back to documented defaults.

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
    "confirm": ["Enter", "E"],
    "cancel": ["C"],
    "menu": ["I", "Escape"],
    "pause": ["Space"]
  }
}
```

## rules.json

Global rules and feature flags.

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

```json
{
  "version": 1,
  "id": "overworld_gaia",
  "name": "Gaia Overworld",
  "world": "gaia",
  "width": 64,
  "height": 48,
  "tiles": [
    "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
    "~~~~..^^....~~~~..^^....~~~~....",
    "~~~~..^^....~~~~..^^....~~~~...."
  ],
  "legend": {
    "~": {"tile": "water", "passable": false},
    ".": {"tile": "grass", "passable": true},
    "^": {"tile": "mountain", "passable": false}
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
      "name": "Elder",
      "pos": [10, 12],
      "sprite": "elder",
      "script": "elder_dialog"
    }
  ],
  "shops": [
    {"id": "corner_store", "pos": [8, 6]}
  ]
}
```

## entities/encounters.json

Encounter tables referenced by maps.

```json
{
  "version": 1,
  "tables": [
    {
      "id": "gaia_grasslands",
      "entries": [
        {"enemy": "imp", "weight": 60, "count": [1, 3]},
        {"enemy": "wolf", "weight": 40, "count": [1, 2]}
      ]
    }
  ]
}
```

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
    {"type": "set_flag", "flag": "world.intro_complete"},
    {"type": "give_item", "item": "potion", "qty": 1}
  ]
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

## ui/battle.json

Defines battle UI layout and panel behavior. Panels can be anchored or flexed to adapt
to terminal sizes.

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
      "crystal_fire": "in_progress"
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
        "text": "Seek the crystal in the Ember Caverns."
      }
    ]
  }
}
```
