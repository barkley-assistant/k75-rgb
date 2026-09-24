# K75 RGB — WORKING PROTOCOL (verified 2026-09-24)

## The change + save POC (both VERIFIED)

All via report **0x09** (feature report, vendor interface /dev/hidraw1, `258a:019d`),
**519-byte** payload frames. Report ID byte `0x09` prepended to frame.

| Step | cmd byte | Frame | Handler (disasm) | Effect |
|------|----------|-------|------------------|--------|
| 1. set color | `0x0a` | `[0x0a, R,G,B, R,G,B, ...]` | fcn.00009308 (op 0x54) | writes config/color region |
| 2. apply | `0x0b` | `[0x0b, 0,0,0, ...]` | 0x7b40 → fcn.000029cd | pushes color to PWM |
| 3. save | `0x06` | `[0x06, 0,0,0, ...]` | fcn.00007393 (op 0x56) | commits to flash |

Sequence (order matters): `0x0a` → `0x0b` → `0x06`.

**Verified:** red `FF 00 00` set, applied, and persisted across power-cycle AND 2.4G
wireless mode.

## Command jump table (@0x7a9f, index = cmd - 0x03, stride 3) — CORRECTED 2026-09-24

| cmd | handler | meaning |
|-----|---------|---------|
| 0x03 | 0x7acf sub-dispatch on 0x08FC | flash ops 0x62/0x5e/... |
| 0x04 | fcn.00008402 | flash READ (op 0x52) |
| 0x05 | fcn.00008fb9 | flash sub-op = data[5] + 0x6e |
| 0x06 | fcn.00007393 | **flash save** (op 0x56) |
| 0x07 | (exit) | no-op |
| 0x08 | fcn.00007108 | direct per-LED RGB (mul #0x12 stride) |
| 0x09 | (exit) | no-op |
| 0x0a | fcn.00009308 | **config write** (op 0x54) |
| 0x0b | 0x7b40 | **apply color → PWM** |
| 0x0c | 0x7b40 | apply |
| 0x0d | 0x7b40 | apply |
| 0x0e | (exit) | no-op |

Note: the earlier published table had 0x08/0x0a handlers swapped — corrected from
the M6 decode. The color-change path runs through cmd 0x0a (config write).

## Config region (from save handler fcn.00007393)

- Save copies CODE table @0xAD7A → XDATA 0x0DB2+ (the live config serialization).
- Default profile: CODE @0xA418, 72 bytes, mode byte 0x35 at +0x0E.
- WARNING (live-verified): writing a malformed config via 0x0a + saving via 0x06
  **persists garbage** that can wedge the lighting engine (all LEDs off, Fn keys
  unresponsive). Recovery: **Fn+Esc hold 3s = factory reset** (restores factory
  rainbow + sane config).

## Key state registers (XDATA)

- `0x0F1D..0x0F20` = current color `[R,G,B,flags]`
- `0x0F54` = lighting mode command byte (0x01 solid, 0x25/0x35/0x45/0x55 modes 1-4)
- `0x0F3F` = effect/parameter register
- `0x0F22` = mode flag register
- `0x0F3D`/`0x0F3E` = apply/config-complete flags
- `0x0F80` = flash op launcher (0x52 read / 0x54 write / 0x56 save / 0x5e / 0x62 / 0x6a)
- `0x0F5B` = arm flag (0x5A = armed)
- flash write engine: fcn.0000da34
- `0x1130+` = 20-byte staged command block (filled by fcn.000096ce; 0x0F7F=1 gates
  fcn.0000b684: [0x1130]==0x5A → register block, ==0x13 → apply)

## KNOWN REMAINING

- **Effects/brightness/speed over USB**: config region is writable (cmd 0x0a) but
  the K75 config layout differs from the F11 template family — offsets unpinned.
  See M2-PLAN.md M6 for the effect-command pipeline and next probes.
- **Case/side underglow light**: separate LED zone, NOT controlled by 0x0a/0x0b/0x06.
  Manual: FN+Tab toggles side light.

## Recovery (proven)

- **Soft:** Fn+Esc (hold 3s) = factory reset — fixes wedged lighting config.
- **Full:** firmware fw/k75_full.bin MD5 be1f5410a2f97d4143b96ed56fbb1c11,
  bootloader fw/k75_boot.bin MD5 3e0ebd0c440af5236d7ff8872343f85d,
  ISP re-flash via sinowisp (0x75 enter → 0x55 enable → 0x52 read / 0x57 write).