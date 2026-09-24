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

## Effect table (CONFIRMED — ground truth from fw/k75_full.bin)

`0xDB76` = 16 effect codes (read via `movc`, indexed by effect counter):

```
01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13
#1 #2 #3 #4 #5 #6 #7 #8 #9 #10 #11 #12 #13 #14 #15 #16
```

- Skips `06`, `0e`, `12` — not valid effects.
- Loaded into `0x0F83` (effect byte), then `0x0CC7` (effect selector 1-16).
- Effect engine `fcn.0000b684` reads per-effect flags `0x0F8F/0x0F90/0x0F93/0x0F94/0x0F97/0x0F98`
  and a 16-bit counter `0x0F3B:0x0F3C`, dispatching op codes `0x10/0x12/0x16/0x1c` via `fcn.0000baa2`.

## Mode system (DECODED — full command → mode-flag map)

`fcn.00000200` (lighting command dispatcher) decodes `0x0F54` → `0x0F22` (mode flag):

| 0x0F54 (cmd) | 0x0F22 (mode flag) | meaning |
|--------------|---------------------|---------|
| 0x01 | 0x14 or 0x24 | solid/static |
| 0x25 | 0x00 | mode 1 |
| 0x35 | 0x01 | mode 2 |
| 0x45 | 0x02 | mode 3 |
| 0x55 | 0x03 | mode 4 |

Sub-flags OR'd into 0x0F22:
- `0x0F3F` == 0x11 → |= 0x04; == 0x12 → |= 0x08; == 0x22 → |= 0x0C
- `0x0BCB` == 1 → |= 0x30; == 2 → |= 0x40
- `0x0F3F` == 0x22 (with 0x0BCB==2) → |= 0x50/0x60/0x70/0x80
- `0x0F50` == 1 → |= 0x80 (brightness channel)

Brightness register `0x0F64` set to 0x3C (default 60) on solid-mode entry.

## Register engine (DECODED)

Report 0x06 `'S' 0x01 [reg] [b0 b1 b2 b3]` = register write; `'R' 'V'` = register read.

`fcn.0000b7d2` (register engine):
- register selector `0x0EFF`; data buffer at `0x0F07`.
- `0x11C1 = 0x5A` (magic), `0x11C2 = op`.
- **Register index N maps to XDATA address `0x11C0 + N`** (via `fcn.0000b800`: `0x11C3 = N`, addr = `0x11C0 + N`).
- Register table lives at `0x11C1+` (also read at 0x6efd/0x9467/0xa281/0xb16f/0xba6d/0xbaa2/0xd9a4).

Report 0x09 command set = flash/config channel (op codes 0x52/0x56/0x5e/0x6a), NOT the
live lighting mode. Live mode/brightness/speed is set via the register protocol
(report 0x06) OR internally (Fn-key / profile load).

## OPEN QUESTION (blocks full M2)

How does a HOST command reach `0x0F54`? Report 0x09 command set (0x03-0x0d) is
flash/config programming (0x06 save, 0x05 flash-op, 0x0a per-key, 0x0b apply).
`0x0F54` is set internally by the mode engine (fcn.0000100e) and Fn-key path.
Candidate host path: report 0x06 'S' register-SET → 0x0F07 config buffer → 0x0F3E|=0x10.
NEEDS EMPIRICAL CONFIRMATION.

1. Full 16-effect list (what does effect 0..15 look like).
2. Brightness write path (0x0F64) over report 0x09.
3. Speed write path (0x0F65).
4. How command byte 0x0F54 selects effect vs solid.
5. Side/case light (separate zone — SETUP handler fcn.0000131c).