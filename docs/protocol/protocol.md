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

The complete feature report is 520 bytes: report ID `0x09` at offset 0,
command at offset 1, and 518 further bytes. The receiving transfer state is
stored at XDATA `0x1150` (not a payload byte); firmware stages each eight-byte
USB chunk from `0x1100` into `0x08FA + report_offset`. The command at host
offset 1 reaches the internal dispatcher at `0x08FB` once the transfer ends.
The command-`0x08` RGB data begins at **complete report offset 8**, not at
payload offset 1. See [the traced ingress path](protocol-audit.md).

### Command table (live-verified where marked)

| cmd | Meaning | Notes |
|---|---|---|
| 0x03 | flash sub-dispatch | sub-op from payload |
| 0x04 | **flash read / internal config reload** (op 0x52) | **FORBIDDEN**: live-tested lights-off failure not restored by known USB commands. Fn+Esc restored the device. Not a host-visible read. |
| 0x05 | flash sub-op | |
| 0x06 | **save** (op 0x56) | commits staged data to data-flash. Persistence of a color setting was verified across power-cycle + 2.4G; do not save unvalidated config. |
| 0x08 | **volatile RGB matrix write** | Firmware-traced host ingress; 126 RGB slots at complete report offsets 8..385. A red baseline visibly lit the keys; slot 0 turned Esc green and slot 63 turned the UK `;` key green. The keys reverted to off after about two seconds without refresh; the case stayed rainbow. Not a six-side-LED packet. |
| 0x0a | **staging/config write** | `0x9308` performs a flash-buffer write, not positional per-key RGB. Full red payload + `0x0b` changed key colors live; the new USB ingress trace reaches this branch, but its exact visible-state semantics remain under investigation. |
| 0x0b | **apply** | applies the staged config/colors to the lighting engine. ✅ VERIFIED. |
| 0x0c / 0x0d | apply variants | decoded in dispatcher; 0x0b is the proven one |

### Traced RAM-matrix experiment (slots 0 and 63 visually validated)

`k75 baseline` prints an offline dry run. `k75 baseline --send` sends
one complete, uniform red matrix without saving it. `k75 slot 0 --send`
turned Esc green; `k75 slot 63 --send` turned the UK `;` key next to L
green while leaving the other keys red. The case light stayed rainbow. The
key lights turned off after roughly two seconds. A bounded run of
`k75 slot 63 --send --repeat 16` resent the *same* RAM-only frame every
500 ms; that kept the keys lit during the run, then they turned off after it
stopped. Those two slots agree with the vendor LED-ID table, but the remaining
slots and a firmware-persistent mode have not been verified.

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
[test-session-2](../../analysis/test-session-2.md) and the
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

## 5. USB-transfer and matrix buffers (XDATA)

| Address | Meaning |
|---|---|
| 0x1100..0x1107 | Eight-byte USB data-chunk source used by the report-0x09 staging loop. |
| 0x1108..0x110F | Eight-byte USB response workspace. |
| 0x1149..0x114E | Report/interface selection and requested transfer length used by the USB setup paths. |
| 0x1150 | USB-transfer state (`0x0B` for report-0x09 receive), **not** a host packet byte. |
| 0x08FA.. | Staged report and internal render scratch. On completed report-0x09 reception, `0x08FB` is the command byte, while `0x0902..0x0A7B` is the command-0x08 matrix source. |
| 0x0379..0x04F2 | Internal 126-slot RGB matrix. |
| 0x1130 | Register-block workspace (`[0x5A, cmd, ...]` consumed by `0xBBCD`). |
| 0x11C1 | Staged command-message workspace; **not** report offset `0xC1`. |
| 0x0EEE..0x0EF0 | Flash-operation entry convention: pointer/offset values stored at handler entry. |

Do not equate absolute XDATA addresses with offsets in a 520-byte host report;
the eight-byte staging copy is the reason offsets 8..385 can be assigned to
host RGB data. Older notes treating `0x1100..0x1303` as one contiguous DMA
frame, or `0x1150` as host `payload[0]`, are superseded.


## 6. Proven recovery

Fn+Esc (hold ~3s) = factory reset of the lighting engine. Details and the
nuclear option in [recovery.md](../reference/recovery.md).

## 7. Hard rules

1. Every packet sent must trace to a live capture or a disassembly offset.
2. Never send flash-erase (0x45) on the ISP channel.
3. Never send report-0x09 command 0x04; an ACK is not evidence of safe behavior.
