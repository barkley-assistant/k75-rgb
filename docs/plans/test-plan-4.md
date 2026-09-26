# Test Plan 4 — effect map with keys lit, persistence re-test, save re-verification

Prepared 2026-09-26 after batch 3 results. Everything uses verified packets
(`0x08` matrix, `0x0a`/`0x0b`/`0x06` save) or the keyboard's own Fn combos.
Recovery: **Fn+Esc hold ~3 s** at any point.

```sh
cd tools/hidra_probe && cargo build --release
```

All sends need `sudo`. Offline previews are safe without it.

---

## Test A — cleanup: return to factory

1. **Fn+Esc hold ~3 s** — factory reset (rainbow wave keys + case).
2. Confirm: keys rainbow wave, case rainbow wave.

This guarantees a known start state for the effect walk (batch 3's walk
started from a mid-cycle position, which made the stop numbering ambiguous).

## Test A — RESULT (2026-09-26): PASS

Fn+Esc reset → keys rainbow wave + case rainbow wave. Known start state
confirmed.

## Test B — RESULT (2026-09-26, clean run): case cycle pinned; keys never left rainbow

Ran after Test A (clean state). One `matrix baseline --send` (keys flashed
red ~2 s), then Fn+Tab once per stop:

| stop | case | keys |
|---|---|---|
| 1 | no visible change (strobe, caught between flashes) | rainbow wave |
| 2 | red | rainbow wave |
| 3 | green | rainbow wave |
| 4 | blue | rainbow wave |
| 5 | yellow | rainbow wave |
| 6 | purple | rainbow wave |
| 7 | cyan | rainbow wave |
| 8 | white / ice-blue | rainbow wave |
| 9 | pulse (breathing colours) | rainbow wave |
| 10 | off | rainbow wave |
| 11 | rainbow wave (wrap) | rainbow wave |

**Findings:**

1. The case cycle is an **11-stop ring**, now pinned from two independent
   walks (batch 3 + this one), identical order:
   `cyan → white → pulse → off → rainbow → strobe → red → green → blue →
   yellow → purple → (wrap)`.
2. **The keys never left rainbow wave** across the whole cycle. Batch 3's
   keys-off mid-walk did not reproduce from a clean start — supports the
   red-herring hypothesis (batch 3 started mid-cycle with keys already in
   a bad state) and the runtime size-gate explanation
   (`0x0377:0x0378` vs `0x08F1:0x08F2`).
3. **The money observation did not appear**: no stop showed the staged red
   matrix persistently, and the key-side 16-effect walk decoded from CODE
   table `0xDB76` does not materialise on the keys via Fn+Tab. The
   persistence gate (effect `0x13` + `0x0C4D=0x69`) is therefore NOT
   reachable through this cycle from a clean state. Reverse-order fallback
   deferred — the case-off stop showed no key-side change either.

## Test B — red-herring walk: keys LIT, map all 17 effect stops

The batch-3 walk had keys off, so we only mapped the case side. This time
the keys are staged red first, so we see the key-side appearance too.

1. `sudo ./target/release/k75 matrix baseline --send` — keys flash red ~2 s.
2. **Within the window or right after**, press Fn+Tab once per stop,
   pausing ~3 s at each. Expected order (decoded from firmware `0xDB76`,
   see `../firmware/effect-table.md`): `01 02 03 04 05 07 08 09 0a 0b 0c
   0d 0f 10 11 13` — 16 stops, then wrap. For every stop record:
   - **keys:** static colour? which? / animated (wave/strobe/breathing)? / off?
   - **case:** static colour? animated? off?
3. Count stops until the cycle visibly repeats. Expected: 17 real stops
   (codes 0x00..0x13 skipping 0x06/0x0E/0x12), terminal stop = the
   custom/static matrix effect.

**The money observation — at the terminal stop (right before the wrap):**
do the keys show the **staged red matrix, continuously** (no 2 s expiry)?
That is the `0x0C4D=0x69` persistence gate. If yes:

- send `sudo ./target/release/k75 matrix slot 63 --send` and confirm the
  `;` key turns green and **stays** green on the static board.
- then send `sudo ./target/release/k75 matrix baseline --send` again and
  confirm it returns to all-red and stays.

If the keys go OFF at the terminal stop again, try the reverse order
(Fn+Tab to the last stop first, then send the matrix) and note what holds.
The shared renderer prologue compares `0x0377:0x0378` against
`0x08F1:0x08F2` — a runtime-updated pair — so the walk should also note
whether the last stop's key-side appearance changes after a second
`baseline --send` (re-staging the matrix after the gate has settled).

## Test C — save re-verification (verified POC, current tooling)

The `0x0a`/`0x0b`/`0x06` sequence was user-verified on 2026-09-24 with the
old probes; re-run it through the clean CLI.

1. `sudo ./target/release/k75 save ff0000 --send` — keys should go red
   (staged + applied + saved).
2. Unplug/replug the keyboard. Keys still red after replug? (Persistence
   via flash — expected yes.)
3. Switch to 2.4 GHz wireless (dongle). Keys still red? (Expected yes.)
4. `sudo ./target/release/k75 save 0000ff --send` — keys blue; replug;
   still blue? (Confirms the save isn't a one-off.)
5. Optional (pattern re-verification): create a small pattern file (one
   `RRGGBB` per line, slot order) and `sudo k75 save --pattern file --send`.
   Expect the traced slot geometry to hold — this is the first live test of
   `from_slots`.
6. Finish with Fn+Esc to restore factory, or save a colour you actually
   want as the daily state.

Note: `save` writes flash. The case light state is not part of this path —
expect the case to keep whatever animation it was on.

## Test D — case determinism: reset + N frames

Does the case always start its frame-stepped cycle at the same stage?

1. Fn+Esc reset. Confirm case wave.
2. `sudo ./target/release/k75 matrix baseline --send --repeat 20 --interval 500`
   (~10 s stream, 20 case steps). Note the final case stage.
3. Fn+Esc again. Repeat with `--repeat 10`. Does the case end up in the
   *same relative stage* after the same number of frames? (i.e. stage
   sequence is deterministic from reset)
4. Optional: record video; correlate against the 19-stage timeline in
   `../analysis/video-evidence-2026-09-26/`.

If deterministic: **reset + N frames = case position control**, which we
can encode as `k75 case <step>` once the cycle length is pinned.

## Test E — case cycle length (video)

1. Fn+Esc reset.
2. `sudo ./target/release/k75 matrix baseline --send --repeat 40 --interval 500`
   (~20 s) while recording video of the case strip.
3. Vision-analyse: does the ~19-stage gradient sequence from the batch-3
   video repeat twice? That pins the cycle length and confirms
   repeatability for Test D.

## Safety

- Never send report-0x09 command `0x04` (lights-off wedge) or ISP `0x45`.
- `matrix` commands are RAM-only and transient. `save` writes flash — it is
  the verified path, and Fn+Esc still recovers.
- Record observations verbatim: keys + case state at every stop, timings.