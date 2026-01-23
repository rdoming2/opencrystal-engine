# OpenCrystal TODO

## Foundations
- [ ] Create runtime state machine (title, overworld, battle, menus, events).
- [ ] Implement event executor for core steps (dialog, narration, set_flag, require_flags, give_item).
- [ ] Add content registry (loaders + in-memory indices for maps/entities/events).
- [ ] Implement save/load for demo content.

## UI + Rendering
- [ ] Title screen renderer (ui/title.json).
- [ ] Overworld renderer (tile map + NPCs + transitions).
- [ ] Battle renderer (ui/battle.json + enemy glyph/art).
- [ ] Menu + progress UI (ui/progress.json).

## Battle System
- [ ] Turn-based flow (command selection, targeting, resolve action).
- [ ] ATB timers (wait + active modes).
- [ ] Status effects and damage formulas (derived stats).
- [ ] Victory/defeat flow and rewards.

## World + Events
- [ ] Map transitions (enter/exit dungeons, overworld zoom).
- [ ] NPC interactions with flag gating.
- [ ] Airship unlock flow (demo completion).

## Content + Validation
- [ ] Add missing demo content (town map, shop UI, basic quests).
- [ ] Extend validation for event step payloads.
- [ ] Add content authoring guide.
