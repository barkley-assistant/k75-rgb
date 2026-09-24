# K75 RGB — Reverse-Engineering Status (honest, 2026-09-24)

## What is VERIFIED (live-tested, traceable)

1. **Report 0x06 register protocol** — `fcn.00005001` (@0x5001), fully decoded:
   - `[06, 'S'(0x53), 0x01, REG, b0,b1,b2,b3]` = SET: REG→0x0EFF, 4 bytes→0x0F07 (via fcn.00002877)
   - continuation (payload[1]≠1): 8 bytes/chunk at 0x0F07+offset, at ≥19B sets 0x0F3E|=0x10 (apply)
   - `[06, 'R'(0x52), 'V'(0x56), 0x01]` = READ → identity string. **VERIFIED LIVE**: returns
     `03 02 48 a3 a3 46 90 0f 50 e0 fe 56 a3 e4 53 ff ee 54 ...`
   - `[06, 'R', 'V', 0x02]` → 0x0EDA=2; else 0x0EDA=0xFF

2. **Report 0x09** = bulk upload (519B max), payload[0] echoed via GET 0x09 (status mirror 0x08FA..0x0901).

3. **Report 0x05** = only `[05 75]` (enter ISP) in normal mode; all else ignored.

4. **USB SETUP layer** — `fcn.0000131c` (@0x14a9): bRequest@0x114A, wValueLow@0x1149,
   wValueHigh@0x114B (report type), wLength@0x114D/E. Data stage → 0x1100 buffer.

## Lighting architecture (decoded, not all live-verified)

- `0x0F54` = command byte (0x01=solid, 0x25/0x35/0x45/0x55=modes 1-4)
- `0x0F1D..0x0F20` = color [R,G,B,flags]
- `0x0F22` = mode flag register; `0x0F3F` = mode byte; `0x0F3E` = apply flags (bit 0x10 = config-complete)
- `fcn.000029cd` = color write [R,G,B,flags|0x80] → @0x0F1D → PWM
- `fcn.0000d15f` = magic gate (0x11E0==0x5A → apply vs staged-config)
- `fcn.0000b684` = command engine (0x0F7F gate, 0x1130 magic 0x5A, 0x0F55 activity dispatch)
- `fcn.00006efd` = LED effect table writer (per-key stride 0x16, table @0x0C52+, key index 0x0CC1)
- GPIO/LED port SFRs: 0x91/0x93/0x97/0x99/0x9a (bit-toggled by scan engine)

## THE GAP (why M1 not yet achieved)

The write path is: USB bytes → config buffer (0x0F07 / 0x1100) → command engine
(fcn.0000b684/bbcd/d2f6) → state registers (0x0F54 command, 0x0F1D color) → effect engine
(fcn.000029cd) → PWM.

I have every stage decoded EXCEPT the exact **field-level byte mapping** at the
config-buffer → state-register handoff (which offset in the 'S' 0x07 buffer is the command,
which is R, which is G/B, and what REG index selects the color vs mode register).

This is the one remaining unknown to produce a *correct* M1 packet. I will NOT fire
invented bytes (hard rule).

## Next steps (zero-guessing options)

A. Finish tracing fcn.0000bbcd / fcn.0000d2f6 (the 0x1130/0x1100 config-buffer consumers)
   to close the buffer→register mapping.
B. Bring in the advanced agent the user offered for a second pass on this specific mapping.
C. Empirical but traceable: fire a *documented* sequence of 'S' 0x01 writes with a single
   register index and observe board state (each byte still traces to decoded code; the
   register index is the only variable being probed, not invented packet structure).