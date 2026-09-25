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
