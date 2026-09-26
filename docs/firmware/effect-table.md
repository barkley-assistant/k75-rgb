# Effect system decode — code → renderer map (offline, 2026-09-26)

Decoded from the stock firmware (`fw/k75_full.bin`), cross-checked against
the Fn+Tab handler trace. Not yet visually confirmed per-stop; the live
walk (test-plan-4, Test B) fills the appearance column.

## Dispatch

Scheduler `0x77BD` computes `jmp @a+dptr` into the table at `0x77C4`
(index = effect code register `0x0F83`). Each entry is an `ljmp` into a
3-instruction stub that `lcall`s the effect renderer:

| Effect code | Renderer | Structural signature |
|---|---|---|
| 0x00 | `0xb3a6` | resets anim pointer `0x0EDB:0x0EDC` — restart/steady state; NOT in the Fn+Tab cycle |
| 0x01 | `0x9108` | resets anim pointer; **the factory wave** (loaded on countdown expiry from `0xA40A[0]=0x01`) |
| 0x02 | `0x7605` | hue ramp, palette `0x06A6` (0,1,2,3…, 0A, 0C, 0E…) |
| 0x03 | `0x8838` | — |
| 0x04 | `0xb25f` | — |
| 0x05 | `0x8d9b` | — |
| 0x06 | — (skipped) | table entry jumps straight to the loop tail; Fn+Tab skips it |
| 0x07 | `0xb2b1` | — |
| 0x08 | `0x8e50` | — |
| 0x09 | `0x596b` | hue ramp, palette `0x06A6`, countdown |
| 0x0A | `0x7c85` | — |
| 0x0B | `0x3b13` | hue ramp, palette `0x0726` (0x16, 0x18, 0x1B, 0x1E…), reads `0x0F83` |
| 0x0C | `0xa7b1` | — |
| 0x0D | `0xb303` | — |
| 0x0E | — (skipped) | Fn+Tab skips it |
| 0x0F | `0x6da5` | — |
| 0x10 | `0x831f` | touches colour index `0x0F87` |
| 0x11 | `0x5108` | hue ramp, palette `0x0726`, reads `0x0F83` |
| 0x12 | — (skipped) | Fn+Tab skips it |
| 0x13 | `0x2dac` | **custom/static matrix**: reads `0x0379`, gates on `0x0C4D==0x69`, LED xmit |

## Fn+Tab cycle order (from CODE table `0xDB76`)

```
01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13
```

16 stops, wrapping 0x13 → 0x01. Effect `0x00` is never reached by
Fn+Tab — it exists for the expiry/steady path. This table is the *source*
the mode handlers map into; the skips (06/0E/12) visible in the jump table
match the Fn+Tab skip codes exactly, which is an internal consistency check
on the decode.

## Shared prologue (all renderers)

Every effect renderer begins with the same size-gate comparison:

```
0x0377:0x0378  (staged matrix length)  vs  0x08F1:0x08F2 (runtime pair)
```

Both registers are **runtime-updated** in many places (the renderers
themselves, the scheduler `0x7737`, the matrix handler region `0x5BFE`),
so this is not a constant "matrix must be 21×6" gate. Its exact semantics
are open; it is probably why the first persistence attempt (Test 3) showed
keys going dark at the terminal stop — the gate may not have been satisfied
in the state we staged. Live discrimination: plan-4 Test B, including the
reverse-order variant.

## Palettes

- `0x06A6` (effects 0x02, 0x09): index ramp `00 01 02 03 … 0a 0c 0e 10 …`
  — increasing step → hue rotation (rainbow-family sweep).
- `0x0726` (effects 0x0B, 0x11): ramp starting at `0x16`, smaller steps —
  a slower/different-hue sweep.
- Ramps index the `0x0786` base colour table (traced previously).

## Predictions for the live walk

1. 16 Fn+Tab stops in the `0xDB76` order above; stop 1 = wave (factory).
2. Stops 0x02 and 0x09 look like the same colour-sweep family (palette A).
3. Stops 0x0B and 0x11 look like a related but distinct sweep (palette B).
4. The terminal stop (0x13) is the custom-matrix effect — the persistence
   candidate. Its key-side appearance depends on the shared size gate.
5. Several stops will look near-identical to the eye (same palette family,
   different timing) — matches the earlier "very similar, hard to
   differentiate" observation.