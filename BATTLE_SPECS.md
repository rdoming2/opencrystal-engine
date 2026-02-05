# Battle UI and Interaction Specs (Short)

This document defines the minimal FF-style battle UI and interaction flow for OpenCrystal.
It aims to be prescriptive while keeping content authoring simple.

## 1) UI layout constraints (FF homage)

- Battlefield occupies the top portion of the screen.
- Command area occupies the bottom portion and is split into three columns:
  - Left: enemy list and target selection.
  - Center: command menu (Attack, Magic, Items, Run).
- Right: party list with HP/MP/Readiness/status.
- No custom column widths in content; layout uses code heuristics.
- Panel titles can be hidden on compact terminals.

## 2) Breakpoints (minimal behavior)

- Compact (small terminals):
  - Enemy visuals are glyph-only.
  - Panel titles hidden.
- Standard (>= 110x32):
  - Enemy visuals use ASCII art when available.
  - Panel titles shown.

## 3) Selection and highlighting rules

- Active party member is highlighted in the party list.
- Selected enemy is highlighted in the enemy list and outlined in the battlefield.
- Selected party target is highlighted in the party list and outlined in the battlefield.
- Menu focus is always visible (invert or underline).

## 4) Interaction flow

1. Active party member becomes ready (turn-based or Readiness).
2. Command menu opens with default selection on Attack.
3. Player selects a command:
   - Attack: enemy list focus opens.
   - Magic: spell list opens, then target selection.
   - Items: item list opens, then target selection (if needed).
   - Run: execute immediately.
4. Target selection confirms action.
5. Enemy actions resolve on their turn or when their Readiness fills (depending on mode).
6. Action resolves, UI returns to command or next party member.

## 5) Menu list content constraints

- Magic list shows columns: Spell, MP.
- Items list shows columns: Item, Qty.
- Enemy list shows enemy name and optional status markers.
- Party list shows HP/MP/Readiness/status in a fixed order.

## 6) Battle pause behavior

- Space toggles pause.
- When paused:
  - Display "PAUSE" overlay centered.
- Readiness timers stop.
  - Input ignores command selection except unpause.

## 7) Target rules

- Each action resolves its target rules:
  - Attack: enemy-only.
  - Magic: uses `default_target` and `allowed_targets` from spells.
  - Items: uses item effect rules (ally/enemy/self/all).
- If target becomes invalid (KO, removed), prompt reselect.

## 8) Enemy visual rules

- If breakpoint is compact, show glyph only.
- If standard, prefer ASCII art if defined; otherwise glyph.
- Bosses may define ASCII art; glyph fallback is always supported.

## 9) Minimal config surface (content)

Content creators only configure:

- Command labels (fixed list).
- Spell/item list columns and grouping (fixed defaults).
- Breakpoints and visual behavior (compact vs standard).

Everything else is enforced by engine logic to keep FF-style consistency.
