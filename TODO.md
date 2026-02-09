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
- [x] Implement event executor.
  - [x] Dialog and narration.
  - [x] Flag setting/requirements.
  - [x] Item/equipment grants.
  - [x] Warp actions.
  - [x] Battle starts.
  - [x] NPC show/hide/move/sprite.
- [x] Add content registry.
  - [x] Load all content into indexed registries.
  - [x] Build cross-reference maps (ids to definitions).
- [x] Implement save/load for demo content.
  - [x] Serialize world/party/inventory/flags.
  - [x] Load persistent NPC positions.
- [x] Implement runtime map transitions.
  - [x] Step on transition triggers map load.
  - [x] Preserve player position on target map.
- [ ] Implement NPC interaction runtime.
  - [ ] Dialog vs event selection.
  - [x] Shop opening via dialog actions.
  - [x] Shop purchase flow (currency, inventory updates).
  - [ ] Shop sell flow (convert items/equipment to currency).
- [x] Add map treasure chests (loot + opened flags).
- [x] Implement save/load system.
  - [x] Define save file schema + versioning.
  - [x] Serialize world/party/inventory/flags.
  - [x] Load persistent NPC positions.
  - [x] Add save slot management.
  - [x] Enforce save rules (map allow_save + save_points).
  - [x] Autosave slot after transitions.

## UI + Rendering
- [x] Title screen renderer.
  - [x] Layout from `ui/title.json`.
  - [x] ASCII logo rendering and menu highlight.
  - [x] Logo line palettes via `ui/title.json`.
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
- [x] Menu + gameplay stats UI.
  - [x] Status menu panels from `ui/gameplay_stats.json`.
  - [x] Journal/quest display.
  - [x] Party management UI.
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
- [x] Add startup content chooser when `--content` is omitted.

## Battle System
- [x] Turn-based flow (command selection, targeting, resolve action).
- [x] Readiness timers (wait + active modes).
- [x] Battle pause toggle (freeze progression).
- [ ] Status effects and damage formulas (derived stats).
- [x] Optional front/back row rules (battle + menu toggles).
- [x] Status persistence rules and overworld poison ticks.
- [ ] Implement command-specific behaviors (steal, throw, pray, parry, cover).
- [x] Victory/defeat flow (spoils, level-ups, modals).
- [x] Scan reveals enemy HP.
- [x] Define battle rules schema (damage, hit, crit, Readiness speed).
- [x] Define status effects schema (buffs/debuffs, durations).
- [x] Define elemental affinities and trait interactions.
- [x] Define skills/abilities schema (non-spell actions).
- [x] Add ability costs (hp/mp/currency/item/death/random).
- [x] Add readiness speed setting to main menu.
- [ ] Add localization support for UI strings and command labels.

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
- [x] Implement overworld vehicle traversal (boarding, movement constraints, persistence).
- [x] Define NPC schema (behavior, dialog, schedules).
- [x] Move event execution logic to engine (apply_event_step).
- [x] Add EventExecutionResult enum for UI action communication.
- [x] Add world state to GameRuntime for warp support.
- [x] Implement on_enter trigger for map load events.
- [x] Implement on_step trigger with zone support.
- [ ] Implement NPC script event triggers.
- [x] Move WorldState from CLI to GameRuntime.
- [x] Implement quest resolver (flag-driven steps and journal updates).
- [x] Add quest step reveal flags (show_flag) and acquisition gating.
- [ ] Implement NPC roaming (persisted positions).
- [x] Implement event-driven NPC controls (show/hide/move/set_sprite).
- [x] Implement inn rest feature (dialog action `rest_party`).
- [x] Add configurable NPC interaction range (default 1 tile).
- [x] Add event action to teach spells (learn_spell).
- [x] Add fast travel menu (free and paid variants).

## Content + Validation
- [x] Add missing demo content (town map, shop UI, basic quests).
- [x] Expand demo content (four rune dungeons, job unlocks, bonus dragon).
- [x] Move rune boss triggers to NPC interactions.
- [x] Align job progression mode with per-job leveling and UI.
- [x] Add boss NPC flash/vanish and immediate dialog event trigger.
- [x] Add job unlock narration in boss events.
- [x] Extend validation for event step payloads.
- [x] Add content authoring guide.
- [x] Add build tool for schema stubs, upgrades, and project scaffolding.
- [x] Add `ui/menu.json` schema + validation.
- [x] Add `party.json` schema + validation.
- [x] Extend map schema for saving.
  - [x] `allow_save` flag for main menu saving.
  - [x] `save_points` coordinates (always valid save spots).
- [x] Add encounter_rate to maps schema.
- [ ] Finalize save schema requirements and versioning rules.
