# OpenCrystal TODO

## Runtime
- [x] Create runtime state machine.
  - [x] Title state.
  - [x] Overworld state.
- [ ] Battle state.
- [x] Menu state.
  - [x] Store menu focus (left list vs right pane).
  - [x] Track active submenu and selection.
  - [x] Pause overworld updates while menu open.

- [x] Party roster + stats pipeline.
  - [x] Predefined party loading.
  - [x] Create-mode party builder.
  - [x] Create-mode job selection.
  - [ ] Rename menu/action for story joins.

- [x] Event state.
- [ ] Implement event executor.
  - [x] Dialog and narration.
  - [ ] Flag setting/requirements.
  - [ ] Item/equipment grants.
  - [ ] Warp and battle starts.
  - [ ] NPC show/hide/move/sprite.
- [x] Add content registry.
  - [x] Load all content into indexed registries.
  - [x] Build cross-reference maps (ids to definitions).
- [ ] Implement save/load for demo content.
  - [ ] Serialize world/party/inventory/flags.
  - [ ] Load persistent NPC positions.
- [x] Implement runtime map transitions.
  - [x] Step on transition triggers map load.
  - [x] Preserve player position on target map.
- [ ] Implement NPC interaction runtime.
  - [ ] Dialog vs event selection.
  - [x] Shop opening via dialog actions.
  - [ ] Shop purchase flow (currency, inventory updates).
- [ ] Implement save/load system.
  - [ ] Define save file schema + versioning.
  - [ ] Serialize world/party/inventory/flags.
  - [ ] Load persistent NPC positions.
  - [ ] Add save slot management.
  - [ ] Enforce save rules (map allow_save + save_points).

## UI + Rendering
- [x] Title screen renderer.
  - [x] Layout from `ui/title.json`.
  - [x] ASCII logo rendering and menu highlight.
- [x] Overworld renderer.
  - [x] Tile map rendering.
  - [x] NPC rendering + gating.
  - [x] Transition markers (glyph + palette highlight).
  - [x] Terminal palette tile coloring (theme-aware).
- [ ] Battle renderer.
  - [ ] Battlefield + enemy art/glyph modes.
  - [ ] Command row (enemy list, commands, party list).
  - [ ] Selection highlights.
- [ ] Menu + progress UI.
  - [ ] Status menu panels from `ui/progress.json`.
  - [ ] Journal/quest display.
  - [ ] Party management UI.
- [x] Main menu UI.
  - [x] Two-pane layout (left list, right detail pane).
  - [x] Default party summary panel.
  - [x] Focus switching (confirm enters right pane, cancel returns).
  - [x] Optional menu entries (rules systems + unlock flag gating).
  - [x] Items/equipment/magic/status submenus (stub views).
  - [x] Party/journal/save/settings/exit submenus (stub views).
  - [x] Save entry enable/disable based on map rules.
  - [x] Save-point glyph rendering (use `S`).
- [x] Inventory + equipment UI.
  - [x] Inventory filters + sorting.
  - [x] Item field use (heal/revive).
  - [x] Equipment swap flow with stat preview.

## Battle System
- [ ] Turn-based flow (command selection, targeting, resolve action).
- [ ] ATB timers (wait + active modes).
- [ ] Status effects and damage formulas (derived stats).
- [ ] Victory/defeat flow and rewards.
- [ ] Define battle rules schema (damage, hit, crit, ATB speed).
- [ ] Define status effects schema (buffs/debuffs, durations).
- [ ] Define elemental affinities and trait interactions.
- [ ] Define skills/abilities schema (non-spell actions).

## Magic System
- [ ] Implement job-based spell learnsets (level/tier/item).
- [ ] Track learned spells per actor.

## Progression
- [ ] Implement experience + level-up pipeline.
- [ ] Apply job growth formulas/tables on level-up.
- [ ] Recompute derived stats with `lvl` variable.

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
- [x] Add `ui/menu.json` schema + validation.
- [x] Add `party.json` schema + validation.
- [ ] Extend map schema for saving.
  - [ ] `allow_save` flag for main menu saving.
  - [ ] `save_points` coordinates (always valid save spots).
- [ ] Define encounter rate rules schema.
- [ ] Finalize save schema requirements and versioning rules.
