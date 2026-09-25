# K75 rendering architecture: transient vs persistent matrix

This documents the firmware's full lighting render pipeline so the transient
~2-second behaviour is explained, and the persistence path is identified as a
testable hypothesis rather than an open mystery. Offsets are file/CODE offsets
in `fw/k75_full.bin`. All statements are instruction-derived.

## The three renderers

| Path | Trigger | Source region | Destination | Behaviour |
|---|---|---|---|---|
| Matrix ingest | report-0x09 cmd `0x08` | `0x08FA+8..` | `0x0379..0x04F2` (126 RGB) | one-shot, sets marker `0x0E34=0x5A` |
| Custom/static matrix | effect `0x0F83=0x13` | `0x0379..` | LEDs via `0xda4f` | **continuous** |
| Preset animations | effect `0x0F83 != 0x13` | effect engine | LEDs | continuous |
| Case/side light | separate engine | `0x0F99..0x0F9F` | LEDs via `0xda4f` | continuous, independent |

## Why the `0x08` write is transient

1. `0x7108` writes the 126-slot matrix and sets `0x0E34 = 0x5A` (the "new
   matrix staged" marker).
2. The scheduler at `0x7781` sees `0x0E34 == 0x5A` and calls `0x5AF8`.
3. `0x5AF8` converts the matrix into the effect-table (`0x05F4` region) and,
   at `0x5BEC`, saves the `0x5A` marker into `0x0FB4`, clears `0x0E34`, and
   loads the countdown `0x0F59 = 0xFA` (250).
4. Each scheduler tick (`0x35BB`) decrements `0x0F59`. While it is non-zero,
   `0x7781` keeps re-calling `0x5AF8` to re-render the matrix.
5. When `0x0F59` reaches zero, `0x35C6` sets `0x23.6` and reloads the effect
   code `0x0F83` from CODE table `0xA40A` — i.e. it **hands control back to
   the preset animation engine**. The custom matrix is no longer rendered.

So the ~250-tick window is by design: the staged matrix is a preview, not a
persistent mode. That is exactly the ~2-second observation.

## The persistence path (hypothesis, evidence-cited)

The scheduler's effect dispatch at `0x77B1` builds a jump table from `0x77C4`:

```
effect 0x0F83 value -> target
   0 -> 0x7800 (lcall 0xb3a6)
   1 -> 0x7805 (lcall 0x9108)
   ...
  18 -> 0x782d (lcall 0x7c85)
  19 (0x13) -> 0x7850 (lcall 0x2dac)   <-- custom/static matrix
```

Effect `0x13` (the 16th entry in the `0xDB76` table) dispatches to `0x2dac`,
which:

- checks `0x0377:0x0378` (matrix size) vs `0x08F1:0x08F2`;
- checks `0x0C4D == 0x69` (a "custom matrix active" flag) — if not set, bails
  to the animation fallback at `0x2e77`;
- otherwise falls straight into `0x2dd6`, the continuous matrix renderer.

Therefore a persistent custom-colour display requires, in firmware terms:

1. the matrix populated in `0x0379..` (already proven via `0x08`);
2. `0x0F83 = 0x13` (select the custom/static effect);
3. `0x0C4D = 0x69` (arm the custom-matrix flag).

## Host-reachable register write (the persistence lever)

The report-0x09 engine can reach a register-write path via the `0x1130`
command block, consumed by `fcn.0000BBCD` (`0xBBCD`). It dispatches on a
command byte in the block:

- command `0xAC` → reads the next block byte and writes it to **`0x0F3F`**
  (the effect index), sets `0x2A.4`. (`0xBBDB..0xBBF2`)
- command `0xAA` → writes the next byte to `0x1155`. (`0xBBE0..0xBBE6`)

The block shape is `[0x5A, <cmd>, <value>, ...]` (magic `0x5A` first). This is
the **only traced host-reachable write into the effect system**: `0x0F3F`
(effect index) feeds the effect-code computation that ends at `0x0F83`, whose
value `0x13` selects the custom/static matrix renderer `0x2dac`.

The remaining unproven link is the exact `0x0F3F -> 0x0F83` mapping (which
effect index selects code `0x13`) and whether setting it also arms `0x0C4D
= 0x69`. This is the highest-value next live test, and it is **RAM-only and
reversible** (no save, no reload), so it is safe to probe once per baseline
under the existing rules.

## What is NOT yet proven

- No host-reachable write into `0x0F83` or `0x0C4D` has been traced. The
  `0x0F83` writers are the effect-cycling engine (`0x36xx`, `0x3cxx`), the
  mode dispatch (`0x203d` region), Fn-key handlers, and the config-load path
  (`0x7F1E` loads `0x0F54` from `0xA437`). `0x0F54` itself is written only by
  Fn-key handlers and config load, not by a live USB frame.
- The config path `profile +0x1F -> 0x0F54` (mode) and the mode->effect
  mapping at `0x203d` is the most plausible host route to `0x0F83 = 0x13`, but
  the exact `0x0F54` value that maps to effect `0x13`, and whether writing it
  also sets `0x0C4D = 0x69`, is **not** established.
- Therefore "persistent per-key colour" remains a firmware hypothesis with a
  precise mechanism, not a verified command. The working fallback is bounded
  host streaming (proven): resend the same `0x08` frame every <2s.

## Mode -> effect map (0x203d)

`0x0F54` values dispatch to mode handlers that set `0x0CC7` (a mode index) and
eventually `0x0F83` (effect code) through CODE table `0xDB76`:

```
0x0F54 = 0x01 -> 0x20df (factory default)
        = 0x25 -> 0x20a7
        = 0x35 -> 0x20b3
        = 0x45 -> 0x20bf
        = 0x55 -> 0x20cb
```

`0xDB76` = `01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13` (16 effects;
effect `0x13` is the custom/static one).

## The case/side light (separate)

See `docs/protocol-audit.md` — the case light is an independent animation
generator on `0x0F99..0x0F9F` with no live-USB writer. It is config-driven
via `0x0BBF` (profile `+0x0F`).