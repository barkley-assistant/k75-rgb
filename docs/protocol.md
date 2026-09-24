# K75 RGB — Verified Protocol Reference

Everything in this file has been **live-tested against the real keyboard** or
is instruction-verified against the stock firmware disassembly. Anything
unverified is marked as such.

## 1. Device

- **RedThunder K75**, en-GB (UK ISO) layout — 81-key LED matrix (verified: no
  upper-bound check exists in firmware, so en-GB shares the ANSI matrix).
- USB ID `258a:019d`, "SINO WEALTH Gaming Keyboard". Two interfaces:
  - iface 0 — standard HID keyboard
  - iface 1 — vendor / RGB control (the one all probes talk to)
- Works wired (USB-C) and over 2.4G wireless (same protocol, both verified).

## 2. HID reports

| Report | Size | Direction | Purpose |
|---|---|---|---|
| 0x06 | 1031 B feature | in/out | register protocol ('S' set / 'R' read) |
| 0x09 | 519 B feature + 7 B input | in/out | main config/lighting command channel |
| 0x05 | ISP | feature | flash ISP (programming; sinowisp-verified) |

On hidraw, feature reports carry the report ID as the first byte.

## 3. Report 0x09 — the main channel

Payload = 519 bytes: `payload[0]` = command, `payload[1..]` = data.
Firmware lands the payload at **XDATA 0x1150**; a second staging buffer at
**XDATA 0x08FA** serves the flash-op handlers (data at offset +8).

### Command table (live-verified where marked)

| cmd | Meaning | Notes |
|---|---|---|
| 0x03 | flash sub-dispatch | sub-op from payload |
| 0x04 | **flash read / internal config reload** (op 0x52) | ⚠ NOT a host-visible read. Reloads data-flash → live lighting state. Firing it re-applies whatever config is saved in flash — this caused a lights-off incident when flash held a stale bad config. Do not send unattended. |
| 0x05 | flash sub-op | |
| 0x06 | **save** (op 0x56) | commits the staging buffer to data-flash (~380 B, ends with 0xAA magic at 0x0BC7). ✅ VERIFIED: persists across power-cycle + 2.4G. |
| 0x08 | **direct-LED** | 6×RGB (18 bytes) from frame → XDATA 0x0379 zone. Likely the side/case underglow strip (untested live). |
| 0x0a | **config write** (op 0x54) | writes config data to the flash write-buffer. ✅ VERIFIED: reaches the config engine (mode changes work). |
| 0x0b | **apply** | applies the staged config/colors to the lighting engine. ✅ VERIFIED. |
| 0x0c / 0x0d | apply variants | decoded in dispatcher; 0x0b is the proven one |

### Working sequences

**Change color (VERIFIED, M1):**
```
send feature 0x09: [0x0a, R,G,B, R,G,B, ...]  (RGB triplets across payload)
send feature 0x09: [0x0b, 0, 0, ...]          (apply)
```

**Save / persist (VERIFIED):**
```
send feature 0x09: [0x06, 0, ...]             (save = flash op 0x56)
```

**Change lighting mode via config (VERIFIED, 2026-09-24):**
```
send feature 0x09: [0x0a, <72-byte config image at payload[1..]>]
send feature 0x09: [0x0b, ...]                (apply)
```
The config image's mode byte at offset `+0x0E` (0x35 = rainbow, 0x45 = wave)
takes effect immediately on apply. See [config-region.md](config-region.md).

## 4. Report 0x06 — register protocol

Handled by `fcn.00005001` @ 0x5001. Shared entry with cmd 0x0a (dispatched
at 0x87bf); the 'S'/'R' branches no-op for RGB payloads.

**SET (stages only — never applies by itself):**
```
[06, 'S'(0x53), 0x01, REG, b0, b1, b2, b3]
```
REG → 0x0EFF, 4 data bytes → 0x0F07. Continuation packets append 8-byte
chunks; at ≥19 bytes staged, 0x0F3E |= 0x10.

**READ:**
```
[06, 'R'(0x52), 'V'(0x56), reg]
```
Only reg 0x01/0x02 return anything useful (identity string). Others return
0x55/0xFF. **Dead end for live state reads** — confirmed empirically.

## 5. Frame buffer layout (XDATA)

| Address | Meaning |
|---|---|
| 0x1100–0x114F | header/magic region (below the payload buffer) |
| 0x1150 | payload[0] = command byte |
| 0x1151.. | payload[1..] (RGB data / config image) |
| 0x1155 | payload[5] — effect-flags byte |
| 0x08FA | second staging buffer for flash ops (cmd 0x04 reads here; flash handlers receive r6:r7 = 0x08FA, r5 = 8) |
| 0x1130 | 20-byte register block workspace (`[0x5A, cmd, ...]` consumed by fcn.0000bbcd) |
| 0x11C1 | staged command-message block (19 B: 0x5A magic, type, sub, op, flags, checksum) |
| 0x0EEE–0x0EF0 | flash-op entry convention: r6:r7:r5 stored at entry by every handler |

## 6. Proven recovery

Fn+Esc (hold ~3s) = factory reset of the lighting engine. Details and the
nuclear option in [recovery.md](recovery.md).

## 7. Hard rules

1. Every packet sent must trace to a live capture or a disassembly offset.
2. Never send flash-erase (0x45) on the ISP channel.
3. Probe config writes only with Fn+Esc ready; never fire cmd 0x04 unattended.