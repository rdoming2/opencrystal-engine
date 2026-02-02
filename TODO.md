# OpenCrystal TODO

## Runtime
- [x] Create runtime state machine.
  - [x] Title state.
  - [x] Overworld state.
- [x] Global stat tracking (playtime, currency).
- [x] Battle state.
- [x] Menu state.
  - [x] Store menu focus (left list vs right pane).
  - [x] Track active submenu and selection.
  - [x] Pause overworld updates while menu open.

- [x] Party roster + stats pipeline.
  - [x] Predefined party loading.
  - [x] Create-mode party builder.
  - [x] Create-mode job selection.
  - [ ] Rename menu/action for story joins and preset_rename mode, plus a `party_join` flow that respects preset/preset_rename rename rules when new members join.

- [x] Event state.
- [ ] Implement event executor.
  - [x] Dialog and narration.
  - [ ] Flag setting/requirements.
  - [ ] Item/equipment grants.
  - [ ] Warp actions.
  - [x] Battle starts.
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
- [x] Add map treasure chests (loot + opened flags).
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
- [x] Battle renderer.
  - [x] Battlefield + enemy art/glyph modes.
  - [x] Command row (enemy list, commands, party list).
  - [x] Selection highlights.
  - [x] Battle dialog overlay.
- [ ] Menu + progress UI.
  - [ ] Status menu panels from `ui/progress.json`.
  - [x] Journal/quest display.
  - [ ] Party management UI.
- [x] Main menu UI.
  - [x] Two-pane layout (left list, right detail pane).
  - [x] Default party summary panel.
  - [x] Playtime and currency display.
  - [x] Focus switching (confirm enters right pane, cancel returns).
  - [x] Optional menu entries (rules systems + unlock flag gating).
  - [x] Items/equipment/magic/abilities/status submenus (stub views).
  - [x] Party/journal/save/settings/exit submenus (stub views).
  - [x] Save entry enable/disable based on map rules.
  - [x] Save-point glyph rendering (use `S`).
- [x] Inventory + equipment UI.
  - [x] Inventory filters + sorting.
  - [x] Item field use (heal/revive).
  - [x] Equipment swap flow with stat preview.

## Battle System
- [x] Turn-based flow (command selection, targeting, resolve action).
- [x] ATB timers (wait + active modes).
- [ ] Status effects and damage formulas (derived stats).
- [x] Victory/defeat flow (spoils, level-ups, modals).
- [x] Scan reveals enemy HP.
- [x] Define battle rules schema (damage, hit, crit, ATB speed).
- [ ] Define status effects schema (buffs/debuffs, durations).
- [ ] Define elemental affinities and trait interactions.
- [x] Define skills/abilities schema (non-spell actions).
- [x] Add ability costs (hp/mp/currency/item/death/random).
- [ ] Add ATB speed setting to main menu.

## Magic System
- [x] Implement job-based spell learnsets (level/item/equip/jp).
- [x] Track learned spells per actor.
- [x] Enable menu casting for field magic.
- [x] Add item-based spell learning (learn_spell items).
- [x] Add Magic Equip slots and menu (spell-granting equipment).
- [ ] Add global shared MP magic style.
- [x] Support job-based tier charge tables.

## Progression
- [x] Implement experience + level-up pipeline.
- [ ] Apply job growth formulas/tables on level-up.
- [x] Recompute derived stats with `lvl` variable.

## World + Events
- [x] Map transitions (enter/exit dungeons, overworld zoom).
- [x] NPC interactions with flag gating.
- [x] Airship unlock flow (demo completion).
- [x] Define NPC schema (behavior, dialog, schedules).
- [x] Move event execution logic to engine (apply_event_step).
- [ ] Add EventExecutionResult enum for UI action communication.
- [ ] Add world state to GameRuntime for warp support.
- [ ] Implement on_enter trigger for map load events.
- [ ] Implement on_step trigger with zone support.
- [ ] Implement NPC script event triggers.
- [ ] Move WorldState from CLI to GameRuntime.
- [x] Implement quest resolver (flag-driven steps and journal updates).
- [ ] Implement NPC roaming (persisted positions).
- [ ] Implement event-driven NPC controls (show/hide/move/set_sprite).
- [x] Implement inn rest feature (dialog action `rest_party`).
- [x] Add configurable NPC interaction range (default 1 tile).
- [x] Add event action to teach spells (learn_spell).

## Content + Validation
- [x] Add missing demo content (town map, shop UI, basic quests).
- [x] Expand demo content (four rune dungeons, job unlocks, bonus dragon).
- [x] Extend validation for event step payloads.
- [ ] Add content authoring guide.
- [x] Add `ui/menu.json` schema + validation.
- [x] Add `party.json` schema + validation.
- [ ] Extend map schema for saving.
  - [ ] `allow_save` flag for main menu saving.
  - [ ] `save_points` coordinates (always valid save spots).
- [x] Add encounter_rate to maps schema.
- [ ] Finalize save schema requirements and versioning rules.
