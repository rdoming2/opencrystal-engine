# Battle Mock Data (Example)

This is a minimal content bundle to exercise the battle UI and selection flow.
Each snippet corresponds to an existing schema.

## entities/enemies.json

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
      "loot": [{"item": "potion", "chance": 0.2}]
    },
    {
      "id": "wisp",
      "name": "Wisp",
      "stats": {"hp": 14, "mp": 8, "str": 2, "int": 4},
      "traits": ["undead"],
      "sprite": {"glyph": "*", "palette": "enemy_magic"},
      "art": {"lines": [" .*. ", "( * )", " ' ' "], "palette": "enemy_magic"},
      "loot": [{"item": "ether", "chance": 0.1}]
    }
  ]
}
```

## entities/encounters.json

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

## entities/spells.json

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

## rules.json

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
  }
}
```

## entities/jobs.json

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

## ui/battle.json

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
    "enemies": {"title": "Enemies", "highlight": {"style": "invert", "link_to_battlefield": true}},
    "commands": {"title": "Commands", "items": ["Attack", "Magic", "Items", "Run"]},
    "party": {"title": "Party", "show": ["hp", "mp", "atb", "status"], "highlight": {"style": "underline", "link_to_battlefield": true}}
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
