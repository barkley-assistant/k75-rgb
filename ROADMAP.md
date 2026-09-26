# ROADMAP — K75 RGB control tooling

Feature-completeness tracker. Statuses: ✅ verified live · 🔬 traced,
unverified · ❓ unknown. Progress is a judgement call on how much of the
end-to-end path is proven, not a promise.

## Summary

| Track | Progress | Notes |
|---|---:|---|
| 1 — Core lighting (write / display / save) | ~70% | Write+save+2.4 GHz verified; host effect selection and polish remain |
| Wireless (2.4 GHz) | **answered** | Dongle forensics 2026-09-26: no live lighting channel — save-then-replay only |
| 2 — Case / side light | ~45% | Fn walk + frame-stepping verified; direct colour write not found yet |
| 3 — Platform & safety | ~50% | Recovery + backup solid; dongle untested, control channel traced only |
| **User goal** (write patterns → save → 2.4 GHz) | **✅ 100%** | Verified 2026-09-24; re-verification with current tooling queued |

Critical path: **Test B** (persistent matrix + effect appearance map) →
**save re-verification** → **host effect selection** → GUI.

## Track 1 — Core lighting

| # | Feature | Status | Progress |
|---|---|---|---|
| 1.1 | Uniform colour write (cmd `0x0a`/`0x0b`) | ✅ | 100% |
| 1.2 | Per-key pattern write | ✅ (mech.) | 85% — layout pinned + `from_slots` implemented; visual confirm pending |
| 1.3 | Direct matrix display (cmd `0x08`, 126 slots) | ✅ | 100% |
| 1.4 | Key map (slots ↔ physical keys) | ✅ (14 pins) | 90% — rest by table reference |
| 1.5 | Flash save (cmd `0x06`) | ✅ | 100% — re-verification queued |
| 1.6 | Save survives replug + 2.4 GHz | ✅ | 100% |
| 1.7 | Persistent custom matrix (effect `0x13`) | 🔬 | 70% — one live test left |
| 1.8 | Host effect selection | 🔬 | 75% — two-block protocol traced (arm `0x11E0` + execute `0x1130`); USB framing unpinned |
| 1.9 | Effect appearance map (17 stops) | 🔬 | 50% — key-side walk pending |
| 1.10 | Brightness (host) | 🔬 | 30% |
| 1.11 | Speed (host) | 🔬 | 30% |

## Track 2 — Case / side light

| # | Feature | Status | Progress |
|---|---|---|---|
| 2.1 | Case effect walk (Fn-driven, 11 stops) | ✅ | 100% |
| 2.2 | Case stepping via matrix frames (1 frame = 1 step) | ✅ | 100% |
| 2.3 | Case cycle map (~19-stage gradient) | 🔬 | 60% — repeatability unproven |
| 2.4 | Deterministic case positioning (reset + N frames) | ❓ | 0% |
| 2.5 | Case direct colour write | ⛔ | N/A — case is effect-table driven, no direct host writer exists |
| 2.6 | Case off (host-controlled) | ❓ | 0% — maybe a cycle stage |

## Track 3 — Platform & safety

| # | Feature | Status | Progress |
|---|---|---|---|
| 3.1 | Recovery (Fn+Esc) | ✅ | 100% |
| 3.2 | 2.4 GHz matrix display | ⛔ | 0% — descriptor forensics: no `0x09` on dongle; wireless = save-then-replay |
| 3.3 | Control-transfer channel (wValue `0x01`/`0x02`) | 🔬 | 30% |
| 3.4 | Profile config read/write (`0x0A`/`0x0B`) | 🔬 | 20% — deferred (0x04 incident) |
| 3.5 | Stock-firmware backup & restore | ✅ | 100% |

## Milestones

| Date | Milestone |
|---|---|
| 2026-09-24 | Colour write + flash save verified; persists across replug + 2.4 GHz |
| 2026-09-24 | Per-key matrix display verified (slot 0 → Esc, slot 63 → `;`) |
| 2026-09-25 | Persistence mechanism traced end-to-end (`0x0F83=0x13` + `0x0C4D=0x69`) |
| 2026-09-26 | Key map: 14 slots visually pinned, gaps + non-key slots confirmed |
| 2026-09-26 | Dongle forensics: real dongle is `258A:0150`; no `0x09` lighting report over 2.4 GHz |
| 2026-09-26 | Case light: Fn+Tab walks 11 stops; **each `0x08` frame steps the case** |

## Next up

1. **Test B (plan 4)** — red-herring walk: stage red keys, Fn+Tab through
   all 17 stops, confirm effect `0x13` renders the staged matrix
   continuously and record every effect's appearance (1.7 + 1.9).
2. **Save re-verification (plan 4, test C)** — the sequence is now ported
   (`k75 save <colour> --send`); run the replug + 2.4 GHz check live.
3. **Case determinism** — Fn+Esc + N frames → predictable case stage (2.4).
4. **Host effect selection** — pin carrier for `0x5A 0xAC` block (1.8).
5. **Polish** — brightness/speed host writes, GUI. (2.4 GHz live drive is
   closed: no `0x09` on the dongle.)

See [`docs/reference/feature-map.md`](docs/reference/feature-map.md) for the
detailed table with command paths and notes.