# K75 RGB — WORKING PROTOCOL (verified 2026-09-24)

## The change + save POC (both VERIFIED)

All via report **0x09** (feature report, vendor interface /dev/hidraw1, `258a:019d`),
**519-byte** payload frames. Report ID byte `0x09` prepended to frame.

| Step | cmd byte | Frame | Handler (disasm) | Effect |
|------|----------|-------|------------------|--------|
| 1. set color | `0x0a` | `[0x0a, R,G,B, R,G,B, ...]` | fcn.00007108 | writes per-key RGB table |
| 2. apply | `0x0b` | `[0x0b, 0,0,0, ...]` | 0x7b40 → fcn.000029cd | pushes color to PWM |
| 3. save | `0x06` | `[0x06, 0,0,0, ...]` | fcn.00007393 (op 0x56) | commits ~380B to flash |

Sequence (order matters): `0x0a` → `0x0b` → `0x06`.

**Verified:** red `FF 00 00` set, applied, and persisted across power-cycle AND 2.4G
wireless mode.

## Command jump table (@0x7a9f, index = cmd - 0x03, stride 3)

| cmd | handler | meaning |
|-----|---------|---------|
| 0x03 | sub-dispatch on 0x08FC | multi-purpose |
| 0x04 | fcn.00008402 | — |
| 0x05 | fcn.00008fb9 | — |
| 0x06 | fcn.00007393 | **flash save** (op 0x56) |
| 0x07 | (skip) | — |
| 0x08 | fcn.00009308 | — |
| 0x09 | (skip) | — |
| 0x0a | fcn.00007108 | **per-key color write** |
| 0x0b | 0x7b40 | **apply color → PWM** |
| 0x0c | 0x7b40 | apply (also flash op 0x5e/0x6a via 0x08FC) |
| 0x0d | 0x7b40 | apply |

## Key state registers (XDATA)

- `0x0F1D..0x0F20` = current color `[R,G,B,flags]`
- `0x0F54` = lighting mode command byte (0x01 solid, 0x25/0x35/0x45/0x55 modes 1-4)
- `0x0F3F` = effect/parameter register
- `0x0F22` = mode flag register
- `0x0F3D` = apply flags
- `0x0F80` = flash op launcher (0x56/0x5e/0x6a)
- `0x0F5B` = arm flag (0x5A = armed)
- flash write engine: fcn.0000da34

## KNOWN REMAINING

- **Case/side underglow light**: separate LED zone, NOT controlled by 0x0a/0x0b/0x06.
  Likely a distinct mode (`0x0F3F`/`0x0F54` combination) or extra key-index range in
  the per-key table. Manual: FN+Tab toggles side light.

## Recovery (proven)

- Full firmware: fw/k75_full.bin MD5 be1f5410a2f97d4143b96ed56fbb1c11
- Bootloader:    fw/k75_boot.bin  MD5 3e0ebd0c440af5236d7ff8872343f85d
- ISP re-flash via sinowisp (0x75 enter → 0x55 enable → 0x52 read / 0x57 write)