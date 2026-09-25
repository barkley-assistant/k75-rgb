# Test Batch 2 — Named Patterns, Per-Key, Side Light

Date: 2026-09-25 · Status: **READY, awaiting live session** · All images pre-built.

Every test in this batch is GO-gated: I fire the command, you describe what the
keyboard shows, we log it, next test. Recovery hatch at any time: `Fn+Esc` hold
~3s = factory reset, or `setcolor2 RR GG BB` to force a known solid color.

---

## 0. Baseline & incident note

Two read-only probes (`rv`, `readcfg`) were run against the device on 2026-09-25.
`readcfg` sends **cmd 0x04 = config reload** — this re-applies the SAVED flash
config, and it visibly **changed the side/case light color** and briefly knocked
the lighting out. Two takeaways:

1. **The side light IS config-driven.** It is in the 72-byte config image.
2. The config reload is a real apply mechanism — `cfgwrite` + a reload applies
   config without the save cycle.

Prime suspect for the side zone: config byte **+0x0F → register 0x0BBF** — the
only config→register path we had not characterized. Firmware dispatches on its
value (0 vs non-zero) through handlers at 0x2164/0x804b/0x8296 that touch system
flags (setb 0x0B/0x11/0x19) and register 0x0F31. **Pattern S is the top priority
of this batch.**

## 1. How each pattern maps to what we know

| Pattern | Config byte | Firmware register | Vendor meaning | Expectation |
|---|---|---|---|---|
| A | +0x1F | 0x0F54 (MODE) | Fn+Tab cycle: 0x25/0x35/0x45/0x55 | 4 wave-family variants |
| B | +0x1B | 0x0F82 → flag 0x0F22 | effect sub-selector (value−1) | animation variants |
| C | +0x0E | 0x0CC8 | wave speed ladder | slow → fast |
| S | +0x0F | 0x0BBF | **behavior switch flag** | **side-light change** |
| K | — | per-key stream | 82 LEDs (vendor matrix) | single keys light |
| R | — | 'S' register file | 16 config registers | unknown — probe |
| L | — | cmd 0x08 → 0x0379 | side zone 6×RGB | side color |

### Key→LED map (vendor KB.ini, authoritative)

`analysis/key-led-map.txt` — LED indices for all 82 entries. Notable anchors:
Esc=0, `=1, Tab=2, CapsLock=3, LShift=4, LCtrl=5, F1=12, Space=35, Enter=81,
RShift=82, Left=83, Delete=84, Up=88, Down=89, Mute=90, Home=91, PgUp=92,
PgDn=93, Right=95. LED 6 is a structural gap (ISO layout hole) — probing it is
part of Pattern K.

## 2. Test sequences

### Pattern S — side light (PRIORITY, 4 tests)

```
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-s01-side-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-s02-side-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-s55-side-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-sff-side-72.bin
```

**Watch the SIDE/CASE light each time** (keys may not change at all).
Expected outcomes: 0x01 vs 0x02 = the firmware's 0-vs-1 dispatch → likely
on/off or color A/B. 0x55 / 0xFF = exotic values, may glitch — that's fine,
that's data.

If any value moves the side light: we've found the side-light byte and the
next step is a 256-value sweep for the color mapping.

### Pattern L — direct side-zone write (2 tests)

```
sudo ./tools/hidra_probe/target/release/sidelight FF 00 00
sudo ./tools/hidra_probe/target/release/sidelight 00 00 00
```

Frame: cmd 0x08, 6×RGB at payload[1..19] → XDATA 0x0379 (best-known layout,
live-verify). If the side light goes red then dark, cmd 0x08 is the direct
side-zone channel.

### Pattern K — per-key (6 tests)

```
sudo ./tools/hidra_probe/target/release/setkey 00 FF 00 00    # Esc → red?
sudo ./tools/hidra_probe/target/release/setkey 0C FF 00 00    # F1
sudo ./tools/hidra_probe/target/release/setkey 23 FF 00 00    # Space
sudo ./tools/hidra_probe/target/release/setkey 51 FF 00 00    # Enter
sudo ./tools/hidra_probe/target/release/setkey 5F FF 00 00    # Right
sudo ./tools/hidra_probe/target/release/setkey 06 FF 00 00    # gap LED 6 — who lights?
```

All non-target keys are sent black, so exactly one key should be lit per test.
This validates the positional triple addressing AND the vendor matrix. If a
test lights the WRONG key, the index offset tells us the real mapping.

### Pattern A — mode sweep (4 tests)

```
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-a1-mode25-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-a2-mode35-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-a3-mode45-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-a4-mode55-72.bin
```

Known: these are the four Fn+Tab states (all rainbow-wave family, near-identical
to the eye). Confirming "same-ish" here is still data — it pins that +0x1F alone
reproduces the Fn+Tab cycle.

### Pattern B — effect-flag sweep (7 tests)

```
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b00-eff-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b03-eff-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b05-eff-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b0b-eff-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b0d-eff-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b11-eff-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-b13-eff-72.bin
```

+0x1B feeds 0x0F82, and the animation flag register 0x0F22 = [0x0F82] − 1.
So these set flags 0xFF/0x02/0x04/0x0A/0x0C/0x10/0x12. Looking for ANY visible
difference — especially anything that looks STATIC or non-wave.

### Pattern C — speed sweep (4 tests)

```
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-c25-speed-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-c35-speed-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-c45-speed-72.bin
sudo ./tools/hidra_probe/target/release/cfgwrite analysis/t2-c55-speed-72.bin
```

Already verified visually in session 1 (slow → fast). Re-run only if the
keyboard state gets weird and we need a speed sanity check.

### Pattern R — 'S' register sweep (16 tests, 1 command each)

```
sudo ./tools/hidra_probe/target/release/regset 00 55 55 55 55
sudo ./tools/hidra_probe/target/release/regset 01 55 55 55 55
... (REG 02..0F, values 55 55 55 55)
```

The 'S' SET channel writes the persistent register file (16 entries, 0x55
defaults). Purpose: find ANY entry with a visible effect. If one dims the
keyboard, it's the brightness path (firmware register 0x0F64 has no config
route — the register file is the remaining candidate). If none do, brightness
is Fn-key-only on the K75 and we document it as a hardware limit.

## 3. Session goals

1. **Side light** (S + L): find the controlling byte/command. Top priority.
2. **Per-key addressing** (K): validate positional stream + vendor matrix.
3. **Modes/effects** (A + B): pin what +0x1F/+0x1B do in named terms.
4. **Brightness** (R): prove or rule out a host route.
5. Anything confirmed gets folded into the final Rust daemon's command set.

## 4. Logging

Every observation goes into `analysis/test-session-3.md` with test ID,
config image, command output, and your verbatim description.