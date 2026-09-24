# K75 Config Region — Layout Hypothesis

Status: HYPOTHESIS for live verification. Anchors from disassembly (addresses =
file offsets in fw/k75_full.bin); semantics INFERRED unless marked VERIFIED.

## Sources

| Source | Address | Content |
|---|---|---|
| Default profile (CODE) | 0xA418, 72 bytes | Factory config image; mode byte 0x35 at +0x0E (VERIFIED by `movc` reads at 0x6272+) |
| Save template (CODE) | 0xAD7A, 72 bytes | Copied to XDATA 0x0DB2 by fcn.00007393; 0xFF slots at offsets 0, 9, 14, 15, 21 |
| Effect index→code | 0xDB76, 16 bytes | 01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13 |
| Mode→effect code | 0xA40A, 24 bytes | 01 20 01 06 03 04 04 00 ... |
| Per-mode recipes | 0xC000/0xC100 (profile 1), 0xC200/0xC300 (profile 2) | 4-byte records |
| Data-flash config region | ~50 bytes @ 0x00-0x32 | Written by cmd 0x0a (op 0x54), saved by cmd 0x06 (op 0x56) |

## Default profile (0xA418) byte-by-byte

```
+00: 02                          profile header/version?
+01..+0D: 00*13
+0E: 35                          MODE byte (0x35 = solid custom)  [VERIFIED anchor]
+0F..+15: 00*7
+16: 63 (=99)                    SPEED? (fast default?)
+17: 01                          ?
+18: 01                          ?
+19..+1B: 00 00 00
+1C: 01                          ?
+1D..+20: 90 31 04 40            32-bit LE value (color? id?)
+21..+47: 14 42 04 47 14 45 0c 45 14 47 0c 40 04 57 0c 47 14 47
          14 47 00 47 04 47 14 47 14 47 14 47 14 47 14 47 10 21
```
The +0x21..+0x47 tail looks like per-key hue/brightness pairs (a rainbow
gradient: low nibbles 0x14/0x0C/0x04/0x00/0x10 = intensity, high values
0x42-0x57 = hue stepping). NOT keymap data (0x47 = 'G' everywhere would be
nonsense as scancodes; as a hue it is a smooth gradient).

## Effect params (C000/C100 records, 4 bytes each, indexed by slot)

Profile 1 (C000 + C100), record = [b0 b1 b2 b3] with b3 the significant byte:
```
slot 0: 0x29   slot 1: 0x35   slot 2: 0x2b   slot 3: 0x39
slot 4: 00 02 00 00  slot 5: 00 01 00 00  slot 6: 0x00  slot 7: 0x1e
(second table: 0x37, 0x10, 0x43, 0x2d, 0x2f, 0x34, 0x38, 0x00)
```
These are the per-effect speed/timing defaults (0x29/0x35/0x2B/0x39 match the
mode bytes used by the Fn-key cycle, VERIFIED values).

## What this means for the driver

mode   = config[0x0E]  (0x35 solid, 0x45/0x55/0x25/0x01 per the 0x0F54 map)
effect = 0x0F3F index into 0xDB76 (host sets via config write + reload)
speed  = config[0x16]? (0x63=99) — verify by write-test
brightness = config[0x17] or [0x18]? (0x01/0x01) — verify by write-test
per-key colors = the +0x21.. tail of the config image (or the separate
126-byte data-flash grid at 0x05F4+36*col)

## Verification plan (needs the user's eyes)

1. Write the factory config image with ONE byte changed (e.g. mode 0x35 →
   0x45) via cmd 0x0a, then apply cmd 0x0b. Observe: rainbow → wave/other.
2. If wedged: Fn+Esc hold ~3s recovers (proven).
3. Binary-search the brightness/speed offsets the same way.
## Ready-to-run test images (2026-09-24)

- config-default-72.bin — factory profile verbatim (control; should reproduce
  the current rainbow/solid state)
- config-mode45-72.bin — mode byte 0x35 -> 0x45 (wave/ripple per the 0x0F54 map)
- config-mode55-72.bin — mode byte 0x35 -> 0x55 (reactive mode)
- config-speed20-72.bin — +0x16 0x63 -> 0x20 (speed candidate)

Run: sudo tools/hidra_probe/target/debug/cfgwrite analysis/config-mode45-72.bin
then watch the keyboard. Wedged? Fn+Esc hold ~3s. Only after a GOOD visual
result: run save_color-style cmd 0x06 to persist.
