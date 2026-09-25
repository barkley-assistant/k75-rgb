# K75 RGB — Working Protocol Reference

This is a working reference, not a complete verified protocol. Read the
[protocol audit](protocol-audit.md) before using any of the older probe commands:
several interpretations of the matrix and config-write paths were disproved by
later live tests. An ACK is not evidence of a lighting change.

## 1. Device

- **RedThunder K75**, user's en-GB (UK ISO) layout. The firmware scans a
  6×16 key matrix, but the vendor LED IDs have not been mapped to host RGB
  slots.
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
The host-command processor reads XDATA `0x1150`. A separate internal
command dispatcher reads `0x08FA` (`0x08FB` selects its command). The transfer
or relationship between the two buffers is **not yet established**; an
internal source offset of 8 is not a host-report payload offset.

### Command table (live-verified where marked)

| cmd | Meaning | Notes |
|---|---|---|
| 0x03 | flash sub-dispatch | sub-op from payload |
| 0x04 | **flash read / internal config reload** (op 0x52) | **FORBIDDEN**: live-tested lights-off failure not restored by known USB commands. Fn+Esc restored the device. Not a host-visible read. |
| 0x05 | flash sub-op | |
| 0x06 | **save** (op 0x56) | commits staged data to data-flash. Persistence of a color setting was verified across power-cycle + 2.4G; do not save unvalidated config. |
| 0x08 | **internal matrix transfer candidate** | `0x7108` writes 21×6×RGB to XDATA `0x0379`; host report path and prefix are **not established**. The former 18-byte side-light probe was invalid and is disabled. |
| 0x0a | **staging/config write** | `0x9308` performs a flash-buffer write, not positional per-key RGB. Full red payload + `0x0b` changed key colors live; exact path from the USB buffer through staging remains under investigation. |
| 0x0b | **apply** | applies the staged config/colors to the lighting engine. ✅ VERIFIED. |
| 0x0c / 0x0d | apply variants | decoded in dispatcher; 0x0b is the proven one |

### Working sequences

**Change key colors (observed live; exact RAM/flash staging semantics still open):**
```
send feature 0x09: [0x0a, R,G,B, R,G,B, ...]  (full payload filled)
send feature 0x09: [0x0b, 0, 0, ...]          (apply)
```
A full-red payload made the keys red after Fn+PgUp had turned the key lights
off. A moving dim-red wave remained visible. This is **not** a proven static
mode or positional per-key RGB packet. The zero-filled `setkey` variant blanked
key lighting and has been disabled.

**Save / persist (VERIFIED):**
```
send feature 0x09: [0x06, 0, ...]             (save = flash op 0x56)
```

**Config-image writes (effects still under investigation):**
```
send feature 0x09: [0x0a, <72-byte image at payload[1..]>]
send feature 0x09: [0x0b, ...]
```
These writes produced visible changes in some live sessions. The initial claim
that profile byte `+0x0E` selected static versus wave was **retracted** after
a factory reset established that the default is already a moving rainbow wave.
Later firmware analysis maps `+0x0E` to speed-related register `0x0CC8` and
`+0x1F` to mode register `0x0F54`. See
[test-session-2](../analysis/test-session-2.md) and the
[protocol audit](protocol-audit.md).

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
| 0x08FA | separate internal command/working region; `0x08FB` selects an internal command. Relationship to the external report is unknown. |
| 0x1130 | 20-byte register block workspace (`[0x5A, cmd, ...]` consumed by fcn.0000bbcd) |
| 0x11C1 | staged command-message block (19 B: 0x5A magic, type, sub, op, flags, checksum) |
| 0x0EEE–0x0EF0 | flash-op entry convention: r6:r7:r5 stored at entry by every handler |

## 6. Proven recovery

Fn+Esc (hold ~3s) = factory reset of the lighting engine. Details and the
nuclear option in [recovery.md](recovery.md).

## 7. Hard rules

1. Every packet sent must trace to a live capture or a disassembly offset.
2. Never send flash-erase (0x45) on the ISP channel.
3. Never send report-0x09 command 0x04; an ACK is not evidence of safe behavior.
