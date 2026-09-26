# Test Plan 3 — key-map completion and persistence (next live batch)

Prepared for the next live session. Everything here uses either the
**verified** `0x08` matrix frame or the keyboard's **own** Fn combos — no
unverified packets. Recovery: **Fn+Esc hold ~3 s** at any point.

Prereqs:

```sh
cd tools/hidra_probe && cargo build --release
# device present:
ls /sys/class/hidraw | head   # K75 should be attached; interface 1 = lighting
```

All sends need `sudo`. Offline previews (no sudo) are safe anywhere.

---

## New firmware findings that shape this batch

1. **Countdown expiry is now fully explained.** `0x35BB` decrements
   `0x0F59`; when it reaches zero `0x35C6` force-loads the effect code
   `0x0F83 = CODE[0xA40A + 0] = 0x01` — the factory wave. That is exactly
   the ~2 s snap-back observed on every single `0x08` send.
2. **Selecting effect `0x13` arms the persistence flag.** The effect-change
   handler at `0x49B9` writes `0x0C4D = 0x69` when the new effect code is
   `0x13` and `0x0C4D == 0`. The `0x2dac` renderer requires exactly
   `0x0C4D == 0x69` to continuously render the RAM matrix `0x0379`.
3. **Effect `0x13` is the last stop of the Fn+Tab cycle** (17 real effects:
   codes 0x00..0x13 minus 0x06/0x0E/0x12).

Prediction: stage the matrix first, then select effect 0x13 via Fn+Tab; the
scheduler dispatches effect 0x13 → `0x2dac` → `0x0C4D == 0x69` → continuous
render of the staged matrix with **no countdown**.

---

## Test 1 — complete the key map (verified path)

For each slot below: `sudo ./target/release/k75 matrix slot N --send`,
then report which physical key (if any) turns green against the red board.
Keys stay lit ~2 s, so read the board within that window.

| # | slot | predicted (unverified) | what this probes |
|---|---|---|---|
| 1a | 30 | F4 | middle of F-row (row 0) |
| 1b | 7  | 1 | number row, col 1 |
| 1c | 55 | 9 | number row, col 9 |
| 1d | 14 | W | QWERTY row, col 2 |
| 1e | 56 | O | QWERTY row, col 9 |
| 1f | 27 | F | home row, col 4 |
| 1g | 51 | K | home row, col 8 |
| 1h | 34 | V | bottom letter row, col 5 |
| 1i | 35 | Space | bottom row, col 5 |
| 1j | 95 | Right | nav cluster, col 15 |
| 1k | 84 | Delete | nav cluster, col 14 |
| 1l | 90 | Mute | top right, col 15 |
| 1m | 10 | `><` | UK ISO key next to LShift |
| 1n | 6  | GAP | structural hole (expect nothing) |
| 1o | 76 | GAP | structural hole (expect nothing) |
| 1p | 100 | NON-KEY | beyond vendor table (dead? case?) |
| 1q | 96 | NON-KEY | beyond vendor table (dead? case?) |

Decision rule: if every predicted key matches (1a–1m), the hypothesis
"host slot = vendor LED ID" holds and the remaining 69 slots are mapped by
table reference. Any mismatch → map that neighbourhood explicitly. GAP and
NON-KEY results tell us whether those slots drive anything at all
(e.g. the case light).

`./target/release/k75 map` prints the whole 126-slot table.

## Test 2 — effect map via Fn+Tab (no packets; pure observation)

Press Fn+Tab repeatedly and name each stop, counting presses from the factory
wave (effect 0x01). Expect 17 distinct stops cycling back. For each stop note:
static / animated, one colour or multi, and roughly which colours. Stop when
the cycle visibly repeats (the wrap 0x13 → 0x00 is the marker).

This pins each effect code 0x00..0x13 to a visual, confirming which stop is
the custom/static matrix effect (predicted: the last one before the wrap).

## Test 3 — persistence recipe (the money shot)

1. Send the stage: `sudo ./target/release/k75 matrix baseline --send`
   (keys flash red ~2 s, then snap back to the wave — expected).
2. **Immediately after**, Fn+Tab through the cycle to the **last stop**
   (effect 0x13, the stop identified in Test 2).
3. Observe for 10+ s: do the keys come back **red and stay red**?

- If they stay red → persistence cracked. Then send
  `sudo ./target/release/k75 matrix slot 63 --send` again and confirm
  the `;` key goes green and **stays** green on the static-red board.
- If nothing / a different animation → the `0x2dac` size gate
  (`0x0377:0x0378` vs `0x08F1:0x08F2`) or `0x0C4D` wasn't armed; we iterate
  (try staging the matrix **while** on effect 0x13: Fn+Tab to last stop
  first, then send).

Also try the reverse order as the fallback experiment:

1. Fn+Tab to the last stop (0x13).
2. `sudo ./target/release/k75 matrix baseline --send`.
3. Observe whether it persists past the ~2 s window (predicted: no, because
   expiry force-loads 0x0F83=0x01 — but worth confirming).

## Test 4 — case/side light via profile +0x0F (careful, later)

Profile byte `+0x0F` feeds `0x0BBF` on config load (traced, untested). This
needs the config path (`0x0A` stage + `0x0B` apply) plus a config reload —
the reload command is exactly what previously wedged the lights
(Fn+Esc recovered). **Do not run in this batch**; keep it queued until Tests
1–3 are done and we can afford the recovery dance. See
[protocol-audit.md](../protocol/protocol-audit.md).

## Safety

- Never send report-0x09 command `0x04` (lights-off wedge) or ISP `0x45`.
- Every send above is the proven RAM-only `0x08` frame; nothing saves.
- Fn+Esc (hold ~3 s) restores factory state at any time.
- Record observations verbatim: which key, how long lit, case-light state
  (rainbow? static? off?) on every test.