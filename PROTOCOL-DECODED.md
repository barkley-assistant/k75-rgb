# K75 RGB Protocol — Decoded (instruction-level, disasm_v2.txt)

## Verified USB channels (live-tested 2026-09-24)

| Report | Direction | Meaning |
|--------|-----------|---------|
| 0x05 | SET | Only `[05 75 00 00 00 00]` = enter ISP. All else silently ignored (handler 0x8765 compares buf==[05 75]). |
| 0x06 | SET | Register protocol via `fcn.00005001` (see below). |
| 0x06 | GET | Returns staged buffer (needs wLength >= staged bytes). |
| 0x09 | SET | Bulk upload, 519-byte max (520 NAKs). payload[0] echoed via GET 0x09. |
| 0x09 | GET | Returns 8 bytes from XDATA 0x08FA..0x0901 (status mirror), via `fcn.00006aee` @0x6aee. |

## Report 0x06 register protocol (fcn.00005001 @ 0x5001)

Pointer pair 0x0EF9:0x0EFA points at received buffer (0x1100). payload layout (payload[0] = first data byte after report ID):

### SET — `'S' (0x53)`
```
payload = [0x53, 0x01, reg, b0, b1, b2, b3]
```
- payload[1] == 0x01 → FIRST packet:
  - payload[2] (reg) → written to 0x0EFF (register selector)
  - payload[3..7] (4 bytes) → copied to XDATA 0x0F00..0x0F03 (via fcn.00002877)
  - 0x0EB5 = 4 (byte counter)
- payload[1] != 0x01 → CONTINUATION: 8 bytes → 0x0F00 + 0x0EB5, 0x0EB5 += 8
- when 0x0EB5 >= 0x13 (19): 0x0EB5=0, 0x0F3E |= 0x10  ("config complete" apply trigger)

### READ — `'R' (0x52) 'V' (0x56)`
```
payload = [0x52, 0x56, arg]
```
- arg == 0x01 → call fcn.00008a93 (build identity string), 0x0EDA = 0
- arg == 0x02 → 0x0EDA = 2 (status)
- else → 0x0EDA = 0xFF
- **VERIFIED LIVE**: `[06 52 56 01]` returns identity string
  `03 02 48(H) a3 a3 46(F) 90 0f 50(P) e0 fe 56(V) a3 e4 53(S)...`

## Lighting apply path (fully decoded)

State registers:
- `0x0F54` = command byte (0x01 = solid/single-color, 0x25/0x35/0x45/0x55 = modes 1-4)
- `0x0F1D..0x0F20` = color [R, G, B, flags]
- `0x0F22` = mode flag register (bit 0x01 = solid mode)
- `0x0F3F` = mode byte (0x11/0x12/0x22)
- `0x0F3E` = config-complete / apply flags (bit 0x10 = "config complete", set by 'S' at >=19 bytes)
- `0x0F55` = activity flag

Apply gate chain (`fcn.0000d15f` @ 0xd15f):
1. flag 0x2a.2 set (apply request) → clear it
2. read 0x11E0, test == 0x5A (magic)
   - == 0x5A → call fcn.0000249b (apply color/mode)
   - != 0x5A → call fcn.0000b490 (process staged config)
3. if 0x0F3F == 0x22 → clear 0x0F55

Single-color apply (`fcn.0000d1dd` @ 0xd1dd):
- gate 0x26.2, command 0x0F54 MUST == 0x01, gate 0x29.7
- read 0x0F1D..0x0F20 → r4,r5,r6,r7 = [R,G,B,flags]
- OR 0x80 into r6 (commit flag)
- call fcn.000029cd (write [R,G,B,flags|0x80] to @dptr=0x0F1D → PWM)

Color write primitive (`fcn.000029cd` @ 0x29cd):
```
movx @dptr,a (r4=R); inc; movx (r5=G); inc; movx (r6=B); inc; movx (r7=flags)
```
= writes 4 bytes [R,G,B,flags] to the address in dptr (callers set 0x0F1D).

## Command processor (fcn.000000ff @ ~0x15c)

Reads 0x0F54, branches:
- 0x01 → 0x0F22 |= 0x01 (solid mode)
- 0x35 → 0x0F22 |= 0x10
- 0x45 → 0x0F22 |= 0x20
- 0x55 → 0x0F22 |= 0x30
- 0x0F50 == 0x01 → 0x0F22 |= 0x80
Then calls fcn.0000a274 (fill) to set mode, 0x0F55 = 0x01, reads 0x0F1D color, applies.

## Memory helpers
- `fcn.00002877` = byte write, dispatch table @0x27f7 by (r5,r3): r3=1 → XDATA (movx), r2:r1 = addr, r5 = len, r7 = value
- `fcn.0000289d` = byte read: r3=1 → XDATA `movx a, @dptr` (r2:r1=addr); r3=0xFE → `movx a,@r1`; else CODE `movc a,@a+dptr`
- `fcn.0000a274` = fill N bytes (r3:r2:r1 = addr, r5 = len, r7 = value)

## ISP protocol (sinowisp-verified, do NOT send flash writes)
- `[05 75 00 00 00 00]` enter ISP (EPROTO = success)
- ISP device = 0603:1020; `[05 55]` enable firmware (to ISP dev), `[05 52 lo hi]` init_read, `[05 5a]` reboot
- `[05 45]` = FLASH ERASE — NEVER SEND.
- Report 0x06 ISP opcodes are ASCII: R=0x52 W=0x57 E=0x45 Z=0x5a U=0x55 u=0x75 r=0x72 w=0x77

## M1 hypothesis (to test — all bytes trace to disasm)

SET all keys red via report 0x06 'S' 0x01:
```
packet1 = [06, 53, 01, REG, R, G, B, FLAGS]   # first: reg selector + 4 bytes → 0x0F00
... continuation packets (8B each) until 19 bytes → 0x0F3E |= 0x10 (apply)
```
The config buffer 0x0F00+ must contain: command 0x01 + color [R,G,B,flags] so the
command processor applies solid color. Exact field offset of command/color within
0x0F00+ is the remaining unknown — needs the 0x0F00+ consumer trace.