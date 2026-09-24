# Live Test Session 1 — 2026-09-24 (results)

Harness: cfgwrite (cmd 0x0a image + cmd 0x0b apply, report 0x09). No saves.
Baseline: Fn+Esc factory reset (rainbow).

| # | Image | Delta | Result |
|---|-------|-------|--------|
| 0 | config-default-72.bin | none (control) | PASS — no change, no wedge |
| 1 | config-mode45-72.bin | +0x0E: 0x35→0x45 | **CHANGED — multi-color WAVE** |
| 2 | mode45-speed20-72.bin | +0x16: 0x04→0x20 | no visible change (see correction below — wrong offset was being tested) |

Note: the corrected +0x1A speed images (mode45-speed05.bin / mode45-speed20.bin)
were built but NOT yet sent live — they're the first tests of Session 2.

## Confirmed
- **+0x0E = mode byte** (VERIFIED live): 0x35 = static rainbow, 0x45 = wave.
- Config write channel + apply works with correct images; no wedge risk with
  factory-identical content.

## Correction
Earlier hypothesis said "0x63 speed candidate at +0x16" — WRONG (row
misreading). Exact profile map (0xA418, 72 bytes):
- +0x0E: 0x35 — MODE (confirmed)
- +0x15: 0x04 / +0x16: 0x04 — brightness/effect pair candidate (untested)
- +0x1A: 0x63 — speed candidate (tested 0x20: no visible change; could be
  non-live or not speed)
- +0x1B: 0x01 / +0x1C: 0x01 — flag candidates
- +0x1F: 0x01
- +0x20..+0x23: 0x90 0x31 0x04 0x40 — 32-bit value candidate
- +0x24..+0x47: per-key gradient tail (hue/brightness pairs)

## Ready for session 2
- mode45-speed05.bin / mode45-speed20.bin (+0x1A variants)
- mode45-br15.bin / mode45-br15max.bin (+0x15 brightness pair)
- mode45-br1c.bin (+0x1C)
