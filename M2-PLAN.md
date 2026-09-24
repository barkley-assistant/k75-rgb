# M2 — Lighting Control Surface Map (in progress)

Goal: a complete, verified map of every lighting control we can drive over USB —
color, brightness, and effect/pattern — so the daemon/GUI can expose them.

## Verified (M1)

- Report 0x09, 519-byte frames.
- `0x0a` per-key color write → `0x0b` apply → `0x06` save (flash).
- Color state: `0x0F1D..0x0F20` = [R,G,B,flags].

## Register map (decoded from disasm_v2.txt)

| XDATA reg | Meaning | Notes |
|-----------|---------|-------|
| 0x0F1D | color R | current color, 4-byte [R,G,B,flags] |
| 0x0F1E | color G | |
| 0x0F1F | color B | |
| 0x0F20 | color flags | 0x01/0x40/0x80 = apply/commit bits |
| 0x0F22 | mode flag | 0x50/0x60/etc = effect state |
| 0x0F23 | mode flag 2 | set to 0x01 on mode change |
| 0x0F3D | apply flags | 0x01/0x02/0x10 = apply triggers |
| 0x0F3F | effect param | mode selector (0x11/0x12/...) |
| 0x0F54 | command byte | 0x01 solid, 0x25/0x35/0x45/0x55 modes 1-4 |
| 0x0F55 | sub-command | |
| 0x0F58 | engine hold/suspend | 0x19 = suspend, 0 = run |
| 0x0F59 | timer | |
| 0x0F5B | arm flag | 0x5A = armed |
| 0x0F64 | brightness | |
| 0x0F65 | speed | |
| 0x0F66 | color index | |
| 0x0F68/6A/6B | params | |
| 0x0F80 | flash op launcher | 0x56/0x5e/0x6a |
| 0x0F83 | effect byte | read from 0xDB76 table |
| 0x0F87 | effect index | 0x04/0x08/0x0C/0x10/0x14 = effects 1-5 |
| 0x0CC7 | effect selector | 0x01/0x02/0x03/0x04 = effect # |
| 0x0CC8 | ? | |

## Effect table

- `0xDB76` = effect data table (movc-indexed), read into 0x0F83.
- Effect index at 0x0F87 maps (stride 4) → 0x0CC7 = 1,2,3,4,...

## TODO (map each)

1. Full 16-effect list (what does effect 0..15 look like).
2. Brightness write path (0x0F64) over report 0x09.
3. Speed write path (0x0F65).
4. How command byte 0x0F54 selects effect vs solid.
5. Side/case light (separate zone — SETUP handler fcn.0000131c).