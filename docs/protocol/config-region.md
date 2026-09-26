# K75 RGB — Config Region

The 72-byte profile at CODE **0xA418** is the default config image the
firmware loads. Historical experiments wrote 72-byte images through report
0x09 commands `0x0a` + `0x0b`. The live color changes are real, but the exact
host-to-profile staging semantics require more tracing. **The table below is
historical and partly superseded**: read [test-session-2](../../analysis/test-session-2.md)
and the [protocol audit](protocol-audit.md) before constructing a new packet.
In particular, `+0x0E` was later traced to `0x0CC8` (speed-related), not the
mode register; `+0x1F` feeds mode register `0x0F54`.

## Byte map (0xA418, 72 bytes, corrected 2026-09-24)

| Offset | Value | Status | Meaning |
|---|---|---|---|
| +0x00 | 0x02 | unknown | profile header/version |
| +0x01..+0x0D | 0x00 | — | zero padding |
| +0x0E | 0x35 | firmware-traced | speed-related register `0x0CC8`; changing this byte altered wave speed, not static-versus-wave mode |
| +0x0F | 0x00 | firmware-traced | behavior flag `0x0BBF`; connection to side light unproven |
| +0x15 | 0x04 | 🧪 candidate | brightness/effect pair A (untested) |
| +0x16 | 0x04 | 🧪 candidate | brightness/effect pair B (untested; earlier misread as speed) |
| +0x17..+0x19 | 0x00 | — | padding |
| +0x1A | 0x63 | 🧪 candidate | **speed** (tested 0x20 → no visible change; maybe non-live or wrong) |
| +0x1B | 0x01 | 🧪 candidate | flag |
| +0x1C | 0x01 | 🧪 candidate | flag |
| +0x1D..+0x1E | 0x00 | — | |
| +0x1F | 0x01 | firmware-traced | mode register `0x0F54`; an isolated USB mode change has not been established |
| +0x20..+0x23 | 0x90 0x31 0x04 0x40 | unknown | 32-bit value (LE: 0x40043190?) — color/timestamp? |
| +0x24..+0x47 | pairs | unknown | per-key gradient tail: (0x14/0x0C/0x04/0x00/0x10, 0x42/0x45/0x47/0x57/0x40/0x21) — reads like hue+brightness pairs for the factory rainbow |

## Live test session 1 (2026-09-24)

Harness: `cfgwrite <image.bin>` = cmd 0x0a (image at payload[1..]) +
cmd 0x0b (apply). No saves — all non-persistent. Baseline: Fn+Esc reset.

| # | Image | Delta | Result |
|---|-------|-------|--------|
| 0 | config-default-72.bin | none (control) | ✅ no change, no wedge |
| 1 | config-mode45-72.bin | +0x0E → 0x45 | ✅ **changed to multi-color wave** |
| 2 | mode45-speed20-72.bin | +0x16 → 0x20 | ❌ no change — but this hit +0x16 (0x04), not the +0x1A speed candidate. Corrected images built, untested. |

The session-1 table **above** records the observation as reported at the time,
**not** a validated static-versus-wave mapping. Session 2 established that
factory default itself is a moving wave; changing `+0x0E` changed its speed.
See [test-session-2](../../analysis/test-session-2.md).

## Ready-made test images (analysis/)

- `mode45-speed05.bin`, `mode45-speed20.bin` — +0x1A variants (wave base)
- `mode45-br15.bin`, `mode45-br15max.bin` — +0x15 variants
- `mode45-br1c.bin` — +0x1C variant
- `config-mode55-72.bin` — +0x0E → 0x55 (maps the 4th mode state)

## Open questions

1. Which byte(s) control **brightness**? (+0x15/+0x16 pair, +0x1B/+0x1C?)
2. Which byte controls **speed**, and does it apply live or only on load?
3. What are the +0x20..+0x23 32-bit and +0x24..+0x47 gradient bytes for?
4. Does the config reload path (save → replug, or cmd 0x04) apply
   brightness/speed where the live apply does not?