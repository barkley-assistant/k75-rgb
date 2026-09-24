# K75 RGB — Plan: what's left

Status checkpoint 2026-09-24. M1 (color change) ✅, persistence ✅, mode
byte ✅. Remaining work broken down into sessions.

## Session 2 — Finish the config map (live, ~20 min with user eyes)

Goal: pin speed + brightness + effect-index offsets in the config region.

1. **Speed** — `mode45-speed05` then `mode45-speed20` (wave base, +0x1A).
   - Acceptance: wave animation visibly changes speed.
   - If null: speed may only apply on config reload — try save (0x06) +
     replug on ONE verified image. Or sweep +0x15/+0x16/+0x1B/+0x1C with
     wide deltas and watch for any visual change.
2. **Brightness** — `mode45-br15`, `mode45-br15max`, `mode45-br1c` (one at
   a time).
   - Acceptance: overall LED brightness visibly changes.
3. **Effect index** — build an image with the effect byte candidates changed
   to force a specific effect; cross-check against the 0xDB76 code table and
   the mode→effect map.
4. **Mode 0x55** — `config-mode55-72.bin` to complete the mode map
   (expected: 4th mode state, likely reactive).
5. When offsets are confirmed: **save test** — write verified image + cmd
   0x06 + unplug/replug (wired + 2.4G) to confirm persistence of mode/bright-
   ness/speed. This also overwrites the stale bad config in flash for good.

## Session 3 — Side/case underglow (live, ~15 min)

- Build a cmd 0x08 probe: 6×RGB (18 bytes) per the decoded contract
  (frame[r5] → XDATA 0x0379). RAM-only, safe.
- Acceptance: the underglow strip changes color. Map which of the 6 RGB
  slots correspond to which physical side LED.

## Session 4 — Driver + CLI (Rust, no user needed)

- `k75-ctl` (or similar) Rust binary over hidra implementing the verified
  protocol: `set-color`, `set-mode`, `set-brightness`, `set-speed`, `save`,
  `factory-reset-hint`.
- Structure the protocol knowledge as a small library crate so the GUI and
  CLI share it.

## Session 5 — Electron GUI

- Rust daemon + HTTP/JSON API (user-endorsed architecture) + Electron
  frontend with color picker, effect list, brightness/speed sliders.
- Widget parity with the driver's verified commands only.

## Session 6 — Repo finalization

- README/docs pass, LICENSE check (GPLv3 present), then make the repo
  public (user deferred this until M2 is done).

## Blocked / open research items

- **Host-visible config read does not exist** — cmd 0x04 is an internal
  reload (probed, documented). Any driver "get current state" must track
  state host-side.
- 32-bit value at +0x20..+0x23 and the +0x24..+0x47 gradient tail semantics
  still unknown (nice-to-have, not blocking).
- Effect-index byte offset unconfirmed (candidates adjacent to mode byte).