# OpenCrystal TODO

## Runtime
- [ ] Create runtime state machine.
  - [ ] Title state.
  - [ ] Overworld state.
  - [ ] Battle state.
  - [ ] Menu state.
  - [ ] Event state.
- [ ] Implement event executor.
  - [ ] Dialog and narration.
  - [ ] Flag setting/requirements.
  - [ ] Item/equipment grants.
  - [ ] Warp and battle starts.
  - [ ] NPC show/hide/move/sprite.
- [ ] Add content registry.
  - [ ] Load all content into indexed registries.
  - [ ] Build cross-reference maps (ids to definitions).
- [ ] Implement save/load for demo content.
  - [ ] Serialize world/party/inventory/flags.
  - [ ] Load persistent NPC positions.
- [ ] Implement runtime map transitions.
  - [ ] Step on transition triggers map load.
  - [ ] Preserve player position on target map.
- [ ] Implement NPC interaction runtime.
  - [ ] Dialog vs event selection.
  - [ ] Shop opening via dialog actions.

## UI + Rendering
- [ ] Title screen renderer.
  - [ ] Layout from `ui/title.json`.
  - [ ] ASCII logo rendering and menu highlight.
- [ ] Overworld renderer.
  - [ ] Tile map rendering.
  - [ ] NPC rendering + gating.
  - [ ] Transition markers (optional).
- [ ] Battle renderer.
  - [ ] Battlefield + enemy art/glyph modes.
  - [ ] Command row (enemy list, commands, party list).
  - [ ] Selection highlights.
- [ ] Menu + progress UI.
  - [ ] Status menu panels from `ui/progress.json`.
  - [ ] Journal/quest display.
  - [ ] Party management UI.

## Battle System
- [ ] Turn-based flow (command selection, targeting, resolve action).
- [ ] ATB timers (wait + active modes).
- [ ] Status effects and damage formulas (derived stats).
- [ ] Victory/defeat flow and rewards.
- [ ] Define battle rules schema (damage, hit, crit, ATB speed).
- [ ] Define status effects schema (buffs/debuffs, durations).
- [ ] Define elemental affinities and trait interactions.
- [ ] Define skills/abilities schema (non-spell actions).

## World + Events
- [x] Map transitions (enter/exit dungeons, overworld zoom).
- [x] NPC interactions with flag gating.
- [x] Airship unlock flow (demo completion).
- [x] Define NPC schema (behavior, dialog, schedules).
- [ ] Implement quest resolver (flag-driven steps and journal updates).
- [ ] Implement NPC roaming (persisted positions).
- [ ] Implement event-driven NPC controls (show/hide/move/set_sprite).

## Content + Validation
- [x] Add missing demo content (town map, shop UI, basic quests).
- [x] Extend validation for event step payloads.
- [ ] Add content authoring guide.
- [ ] Define encounter rate rules schema.
- [ ] Finalize save schema requirements and versioning rules.
