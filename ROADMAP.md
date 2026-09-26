# ROADMAP — K75 RGB control tooling

Feature-completeness tracker. Statuses: ✅ verified live · 🔬 traced,
unverified · ❓ unknown. Progress per track is a judgement call on how much
of the end-to-end path is proven, not a promise.

## Track 1 — Core lighting (write & display)

| # | Feature | Status | Progress |
|---|---|---|---|
| 1.1 | Uniform colour write (cmd 0x0a/0x0b) | ✅ | 100% |
| 1.2 | Per-key pattern write | ✅ (mechanically) | 80% — uniform only visually confirmed |
| 1.3 | Direct matrix display (cmd 0x08, 126 slots) | ✅ | 100% |
| 1.4 | Key map (slots ↔ physical keys) | ✅ (15 pins) | 90% — rest by table reference |
| 1.5 | Flash save (cmd 0x06) | ✅ | 100% (re-verification queued) |
| 1.6 | Save survives replug + 2.4 GHz | ✅ | 100% |
| 1.7 | Persistent custom matrix (effect 0x13) | 🔬 | 70% — one live test left |
| 1.8 | Host effect selection | 🔬 | 40% — carrier unpinned |
| 1.9 | Effect appearance map (17 stops) | 🔬 | 50% — key-side walk pending |
| 1.10 | Brightness (host) | 🔬 | 30% |
| 1.11 | Speed (host) | 🔬 | 30% |

## Track 2 — Case / side light

| # | Feature | Status | Progress |
|---|---|---|---|
| 2.1 | Case effect walk (Fn-driven, 11 stops) | ✅ | 100% |
| 2.2 | Case stepping via matrix frames | ✅ | 100% (1 frame = 1 step) |
| 2.3 | Case cycle map (~19-stage gradient) | 🔬 | 60% — repeatability unproven |
| 2.4 | Deterministic case positioning (reset + N) | ❓ | 0% |
| 2.5 | Case direct colour write | ❓ | 0% — no writer found yet |
| 2.6 | Case off (host-controlled) | ❓ | 0% — maybe a cycle stage |

## Track 3 — Platform & safety

| # | Feature | Status | Progress |
|---|---|---|---|
| 3.1 | Recovery (Fn+Esc) | ✅ | 100% |
| 3.2 | 2.4 GHz matrix display | ❓ | 0% — dongle untested |
| 3.3 | Control-transfer channel (wValue 0x01/0x02) | 🔬 | 30% |
| 3.4 | Profile config read/write (0x0A/0x0B) | 🔬 | 20% — deferred (0x04 incident) |
| 3.5 | Stock-firmware backup & restore | ✅ | 100% |

## Critical path to "feature complete"

The user's original goal — write colour patterns, save to memory, works on
2.4 GHz — is **already verified end-to-end** (tracks 1.1 + 1.5 + 1.6) from
2026-09-24. The current push is:

1. **Test B (red-herring walk)** — verify persistent custom matrix via
   effect 0x13 (1.7), and record all 17 effect appearances (1.9).
2. **Save re-verification** — port 0x0a/0x0b/0x06 into the trusted library
   + CLI, re-run replug + 2.4 GHz check.
3. **Case determinism** — Fn+Esc + N frames (2.4).
4. **Host effect selection** — pin carrier (1.8), enabling the whole
   effect system without touching Fn+Tab.
5. **Polish** — brightness/speed host writes, 2.4 GHz matrix probe, GUI.

See `docs/feature-map.md` for the detailed table with paths and notes.