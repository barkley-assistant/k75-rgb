# Feature map — K75 RGB control, RE status

Last updated 2026-09-26 (post experiment A). Statuses:

- **VERIFIED** = observed on the physical board by the user this or the
  previous session, via a packet traceable to a capture/disassembly offset.
- **TRACED** = firmware path mapped offline, packet constructed from
  disassembly, NOT yet visually confirmed.
- **UNKNOWN** = not yet traced to a host-reachable path.

## Core lighting

| Feature | Status | Path | Notes |
|---|---|---|---|
| Set all keys to one colour | **VERIFIED** | report `0x09`: cmd `0x0a` per-key RGB write + cmd `0x0b` apply | User-observed solid red (2026-09-24). |
| Per-key colour table (patterns) | **IMPLEMENTED** (lib) + uniform visually verified | `0x0a`/`0x0b` path; slot geometry pinned from `fcn.00007108` (stride `0x12` = 6 RGB triples/row, slot `i` at payload `2+i*3`) | `PerKeyColorFrame::from_slots([Rgb;126])` builds arbitrary patterns; only uniform red visually confirmed so far. |
| Direct matrix display (126 slots) | **VERIFIED** | report `0x09` cmd `0x08`, 520-byte frame, offsets 8..385 | Transient ~2 s; `--repeat` keeps it lit. 15 key slots + 2 gaps + 2 non-key slots verified. |
| Flash SAVE (persist) | **VERIFIED** | cmd `0x06` → `fcn.00007393` (op 0x56, ~380 B commit, 0xAA magic) | Red survived unplug/replug **and 2.4 GHz wireless mode** (2026-09-24, user-observed). Re-verification with current tooling pending (legacy probes were error-prone). |
| Persistent custom matrix (effect 0x13 path) | **TRACED** | stage matrix `0x08` + select effect `0x0F83=0x13` → arms `0x0C4D=0x69` → continuous render | First live attempt: keys went OFF at terminal stop, case went breathing. Red-herring re-test queued (test B). |
| Effect selection (host) | **TRACED (two-block protocol)** | Phase 1 (arm): block B at `0x11E0`, value `0x22` → `0x0F3F=0x22` via `fcn.0000249b`; Phase 2 (execute): block A `0x5A 0xAC <idx>` → `0x1155=idx` via `fcn.0000bbcd`, byte-assembled into `0x1130` while armed (`0x0F3F==0x22 && 0x0F54==0x01`). USB framing of phase 1 still unpinned. Control-transfer "AH"/"AZ" channel pinned at `0x14A9`. See `../protocol/effect-selection.md` | Fn+Tab does it natively (keyboard-side, 17 real effects). |
| Effect code map | **VERIFIED** (via Fn+Tab) | `0x0F83` = effect code 0x00..0x13, skips 0x06/0x0E/0x12 = 17 real effects | Key-side appearance of each stop NOT fully recorded (keys were off during last walk). |
| Brightness | **TRACED** | `0x0F82`-family, Fn+PgUp path | Fn+PgUp verified natively (off / 7 levels / on). Host write unverified. |
| Speed | **TRACED** | `0x0CC8` (fed by profile +0x0E) | Fn+←/→ verified natively. Host write unverified. |

## Case / side light

| Feature | Status | Path | Notes |
|---|---|---|---|
| Case effect selection | **VERIFIED** (Fn-driven) | Fn+Tab walks 11 case stops: cyan, white, pulse, off, wave, strobe, red, green, blue, yellow, purple | Case sub-state `0x0F9B`, palette `0x0F9F`. |
| Case animation stepping (host) | **VERIFIED** | `0x08` matrix frame — **1 frame = 1 case animation step** | Frame-synced (experiment A, 2026-09-26). Video shows ~19-stage gradient cycle at 500 ms cadence, hold, then wave. |
| Case direct colour write | **NOT FOUND — likely doesn't exist** | case params load from the per-effect ROM table `0xA40D+` (loader `0x6225-0x627F`); the persistence commit copies case state into the `0x0Dxx` save-image area; no host packet writes `0x0F99..0x0F9F` directly | Case control = effect selection + config profile instead of a direct register write. |
| Case cycle determinism (reset + N frames) | **UNKNOWN** | Fn+Esc ~3 s factory reset, then N frames | Untested; would give deterministic case positioning. |

## Key mapping

| Feature | Status | Notes |
|---|---|---|
| 126-slot matrix ↔ physical keys | **VERIFIED (structure)** | 21×6 firmware chain; 16×6 physical grid (96 positions); 82 vendor LED IDs + 14 gaps; 30 non-key slots (96..125) with no visible LED. |
| Vendor table pins | **VERIFIED (14 keys)** | Esc=0, `;`=63, F4=30, 1=7, 9=55, W=14, O=56, F=27, K=51, V=34, Space=35, Right=95, Delete=84, `\|`=10, plus slot-6/76 gap + 96/100 non-key nulls. |
| Legend corrections (en-GB) | **VERIFIED** | slot 10 = `\|` (vendor said `><`); slot 90 = no physical Mute key on this variant. |
| Remaining 68 keys | **HIGH CONFIDENCE (by table)** | predicted from vendor table; 14/14 key pin checks matched (plus gap/non-key nulls), rest by reference. |

## Persistence & platform

| Feature | Status | Notes |
|---|---|---|
| Save survives replug | **VERIFIED** | cmd `0x06` path (2026-09-24). |
| Save works on 2.4 GHz wireless | **VERIFIED** | user-observed (2026-09-24). |
| `0x08` matrix over 2.4 GHz | **RESOLVED: not exposed** | dongle descriptor (2026-09-26) has no 520-byte report `0x09` — wireless is save-then-replay; see `../protocol/wireless.md`. |
| Recovery | **VERIFIED** | Fn+Esc ~3 s = factory reset; restores rainbow. Never send report-0x05 ISP `0x45`, nor report-0x09 cmd `0x04`. |

## What "feature-complete" still needs

1. **Persistence re-test (test B)** — re-stage red keys, Fn+Tab walk with
   keys lit, find the terminal stop; confirm effect 0x13 renders the staged
   matrix continuously. This is the last piece for "persistent custom
   pattern" via the new path.
2. **Save re-verification** — re-run `0x0a`/`0x0b`/`0x06` with the current
   trusted library + CLI (port from legacy probes), replug + 2.4 GHz check.
3. **Effect appearance map** — record key-side behaviour of all 17 effect
   stops (red-herring walk covers this).
4. **Case determinism** — Fn+Esc + N frames → predictable case stage.
5. **Host effect selection** — pin the carrier for `0x5A 0xAC` block or
   test the control-transfer channel; replaces Fn+Tab for host control.
6. **Brightness/speed host writes** — verify traced registers live.
7. **2.4 GHz matrix support** — probe the dongle's HID endpoints.

Items 1-2 are the critical path for the user's original goal ("write
colour patterns, save, works on 2.4 GHz").