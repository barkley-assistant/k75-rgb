# Live Test Session 2 — 2026-09-25 (baseline re-established)

Baseline: Fn+Esc = RGB flash (red→green→blue) then SLOW rainbow wave, left→right.
This invalidates session 1's "0x45 = wave" claim — the factory state IS a wave.

| # | Image | Delta (+0x0E) | Result |
|---|-------|---------------|--------|
| 1 | config-default-72.bin | none | flicker on apply, final state identical ✅ |
| 2 | config-mode45-72.bin | 0x35→0x45 | same wave, maybe slightly faster |
| 3 | config-mode55-72.bin | 0x35→0x55 | same wave L→R, clearly quicker |

Hypothesis: +0x0E = wave speed class (0x35→flags 0x01, 0x45→0x02, 0x55→0x03).
Prediction: 0x25 (flags 0x00) = slowest.

## Config-load register map (decoded 2026-09-25, instruction-verified)
fcn @ 0x7eed: profile+0x00→0x0F56, +0x1B→0x0F82, +0x1D→0x0F81, +0x0E→0x0CC8,
+0x0F→0x0BBF, +0x1F→0x0F54 (MODE), +0x1E→0x0F50 (channel), +0x07→?.
CORRECTION: +0x0E is the wave speed param (0x0CC8), NOT the mode. Real mode byte
= +0x1F → 0x0F54 (factory 0x01). Brightness 0x0F64 has NO config-load path.
