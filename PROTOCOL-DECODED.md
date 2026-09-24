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
## Effect command grammar (2026-09-24, instruction-verified)

### The 0x66 effect command processor (fw 0x66-0x2xx, inside vector area)
Every branch builds a command block via fcn.0000a274 (ptr=0x0F22, r5=value, r7=OPCODE),
sets [0x0F55]=1 (apply), reloads color [0x0F1D..0x0F20] with one flag bit cleared, then
jumps to the effect-emit path (0x029c / 0x0697 / 0x0236).

Observed effect opcodes (r7 to fcn.0000a274): 0x02, 0x03, 0x04, 0x08, 0x0A, 0x13, 0x15, 0x17, 0x18, 0x1B.
Triggers: [0x0F1F] bit 0x80 -> op 0x08; bit 0x02 -> op 0x1B (clear mode); bit 0x04 -> op 0x03;
bit 0x08 -> mode dispatch (below) then op 0x02; [0x0F41]==1 -> |= 0x80, op 0x04; [0x0F4B] -> op 0x04.

Mode -> flag map (0x0F54 / 0x0F3F / 0x0F50 -> 0x0F22 bits):
  0x0F54==0x01 -> |= 0x01 ; else -> |= 0x02
  0x0F3F==0x11 -> |= 0x04 ; 0x0F3F==0x12 -> |= 0x08 ; 0x0F3F==0x22 -> |= 0x0C
  0x0F54==0x35 -> |= 0x10 ; 0x0F54==0x45 -> |= 0x20 ; 0x0F54==0x55 -> |= 0x30
  0x0F50==0x01 -> |= 0x80
0x1155 is read with `anl #0x07` and contributes effect flags (lighting parameter register).

### fcn.0000bbcd: the [0x1130]==0x5A register-block consumer
20-byte block at 0x1130 (filled from XDATA 0x11E0 by the copy engine):
  [0x1130] = 0x5A (magic)
  [0x1131] = command:
    0xAC -> [0x1132] -> 0x0F3F   (EFFECT INDEX register - host-settable)
    0xAA -> [0x1133] -> 0x1155   (lighting parameter register)
    else -> 0x0F3D |= 0x20
A report-0x09 frame landing `5A AC <effect>` at the command block sets the effect index.

### Copy engine (fcn.0000b893 / fcn.0000b8b4 / fcn.0000d093)
fcn.0000b893: count=r7 -> [0x0F7E], dest ptr = 0x11C1, arm copy.
fcn.0000b8b4: source = XDATA 0x11E0 (staged command register), [0x0F73]=r7.
fcn.0000d093: 16-bit counter decrement per byte; reads via fcn.0000295c (space 1 = XDATA),
writes via fcn.0000289d to dest.
fcn.0000b800: dest = 0x11C0 + (REG+5) where REG = 0x0EFF from 'S' staging.

### fcn.0000a6de: sub-dispatcher (PARTIALLY DECODED)
Stores r7:r5 to [dptr..+2]; byte dispatch on {0x07, 0x05, 0x08, 0x0B, 0x13} -> paths
0xa70f (->0xb986), 0xa71a (->0xb20b), 0xa729 (->0x9f7e), 0xa738 (->0x8690),
0xa744 (->fcn.00009260 = 0x1130 feeder). Called from 0x5853. TODO: entry byte source.

### Analysis-base integrity check
disasm_v2.txt verified byte-consistent with fw/k75_full.bin at the vector region
(0x43 = `ljmp 0x6280` in both). Earlier "0x43 mismatch" was a misreading; the
trusted disasm base is sound.

## Effect architecture (complete, 2026-09-24)

### Effect index -> code -> handler chain
1. Effect index register 0x0F3F.
   Host-settable via the 0x1130 register block: [0x5A, 0xAC, <index>] -> fcn.0000bbcd
   stores [0x1132] into 0x0F3F.
2. 0x0F3F -> CODE table 0xDB76 (16 entries):
     01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13
   -> effect command register 0x0F83.
3. 0x0F83 -> fcn.0000a6de dispatcher (entry stores r7:r5 -> [0x0EE7:0x0EE8]):
     0x07 -> 0xb986   0x05 -> 0xb20b   0x08 -> 0x9f7e   0x0b -> 0x8690
     0x13 -> fcn.00009260 (the 0x1130 command-block feeder; per-key color path)
     else -> ret.  Gated on [0x0BA6] == 0.
4. fcn.00009260: writes r6:r7:r4:r5 -> [dptr], gated on config [0x0C4D] == 0xFF
   or [0x0C4E] == 0x01, reads [0x0C4F], [0x0C51] -> r7:r5, calls fcn.0000976c.

### Effect tables
- 0xDB76 (16 bytes): effect index -> command code (above).
- 0xA40A (24 bytes): mode -> effect-code table used by Fn+| cycle
  (01 20 01 06 03 04 04 00 00 00 00 00 00 99 02 ...).
- 0xC000/0xC100/0xC200/0xC300: 4-byte effect param records (two profiles;
  0xC228 differs: 64 00 08 vs 64 00 04).

### Mode / brightness registers (live XDATA)
- 0x0F54 mode command byte; 0x0F22 mode flags (see map in previous section).
- 0x0F64 brightness (default 0x3C = 60). All five writers firmware-internal.
- 0x0F3D lighting flags; 0x0F3E staged-apply flags.
- 0x1155 lighting parameter register (read with anl #0x07).
- 0x0BA6 gate, 0x0C4D-0x0C51 live config bytes (0x0C4D gate for the
  command-block feeder; 0x0C4F = stride multiplier for per-key builder).

### Remaining unknown for M2
The report-0x09 frame offsets that fill the 0x1130 command block. Known:
fcn.000096ce copies from [0x0F75:0x0F76] (XDATA 0x11E0 in one path) -> 0x1130;
called from fcn.00009260 (effect 0x13) and 0xa778. The per-key builder at
fcn.000057db computes a table address = 0x0575 + index * [0x0C4F].
Next: decode fcn.00001b46 (frame processor) to pin the frame byte -> 0x1130 mapping.

## Flash read command (cmd 0x04) internals (2026-09-24)

fcn.00008402 (cmd 0x04 handler):
- Writes opcode 0x52 to [0x0F5B], arms with 0x5A, polls until cleared.
- frame[9]==0x01 -> firmware writes 0x13 into the frame at offset 10
  (response marker; visible in the GET-0x09 echo).
- 128-iteration loop (counter 0..127):
    * r6:r7 = counter stored to [0x0EF3:0x0EF4] (flash address pair)
    * r5 = frame[counter+8] (loaded but NOT used by fcn.0000da34)
    * r3 = 0x52 (flash READ op)
    * fcn.0000da34: arms [0x0FAE]=5, [0x0FAF]=0x0A (async flash-op flags);
      when the address LOW byte == 1, also calls fcn.0000b9c0 immediately.
    * [0x0F5B] cleared, [0x0BC2]/[0x0BC7] cleared, [0x0BC8]=0xAA at exit.

fcn.0000b9c0 (flash op init): reads address pair, sets op r7 = 0x6E, clears
the pair. 0x6E is in the fcn.0000b35e valid-op set
{0x52,0x5E,0x5F,0x62,0x63,0x66,0x67,0x6A,0x6B,0x54,0x56,0x6E..0x75}.

Async model: the flash op executes via the polled [0x0FAE]=5/[0x0FAF]=0x0A
arm flags, not synchronously. The executor is fcn.0000d8e8 (op dispatch,
calls fcn.0000b355 to store the address and fcn.0000b9c0 to fire).
TODO: decode fcn.0000d8e8 fully to pin where read results land.

Empirical safety note: a zero-filled cmd-0x04 frame runs this entire loop
harmlessly (probe readcfg, keyboard healthy after). frame[9]=1 only adds the
0x13 response marker.

## Frame-pointer chain correction (2026-09-24)

[0x0EE7:0x0EE8] = the FRAME POINTER pair (host-influenced):
- fcn.0000a6de (effect dispatcher) stores r7:r5 -> [0x0EE7:0x0EE8] at entry.
- r7:r5 come from call site 0x5853 in fcn.000057db, which walks a per-key
  table at XDATA 0x0575 + index*[0x0C4F] and loads two bytes from the frame.
- The apply gate fcn.0000218b reads the table at [0x0EE7:0x0EE8] on every
  call (the other agent's M5 was right about this pointer being
  host-controlled; wrong that payload[0..2] sets it directly).

So the host CAN aim the apply gate at its own staged bytes IF the frame
positions the pointer correctly. The mapping frame-bytes -> table address is
in fcn.000057db (0x5822: r7 = [dptr]; r6 = [0x0C4F]; a = r7 * r6 + 0x75,
high = 0x05 + carry, then add a,0x1e [R6 reg]).

fcn.0000d8e8 (flash op dispatch): calls fcn.0000b35e (valid-op set check);
if r7 == 1 -> op 0xE6, clear address pair. The real executor runs from the
polled [0x0FAE]/[0x0FAF] arm flags.

## Data-flash engine (2026-09-24)

Flash op flow: op staged at 0x0F80, arm at 0x0F5B (0x5A -> fire). Main-loop
dispatcher at 0x62d7 polls [0x0F5B]==0x5A -> fcn.0000d8e8 -> fcn.0000b35e
(op validator+executor). Op staging sites: cmd 0x04 -> 0x52 (READ) at 0x840d;
cmd 0x0a -> 0x54 (WRITE) at 0x9313; also 0x5E (0x94fa), 0x6A (0x959a),
0x62 (0x99e9), 0x66 (0x9a83).

fcn.0000b35e valid-op set: 0x52, 0x5E, 0x5F, 0x62, 0x63, 0x66, 0x67, 0x6A,
0x6B, 0x54, 0x56, 0x6E..0x75. Each valid op enters the 0xb38a execution block
at a per-op offset. The block checks XDATA [0x08F2] vs 0xF4 and [0x08F1] vs
0x01, then loops a 16-bit counter at 0x0EDB:0x0EDC (limit 6).

Data-flash transfers run against the XDATA 0x08xx region (the live per-key
color table area): 0x08F1/0x08F2 = the table pointer/limit. This is the
bridge between the ISP data-flash (config region, ~50 bytes addressed
0x0000-0x0032 via [0x0F5E:0x0F5F]) and the live per-key table.

Config flash address pair [0x0F5E:0x0F5F]: [0x0F5F] reset to 0 (0x1edf),
[0x0F5E] a running counter capped at 0x32=50 (0x6366). Config = 50-byte
data-flash region.

## Per-key table geometry (2026-09-24)

- [0x08F1:0x08F2] = 16-bit pointer into the live per-key table region
  (XDATA 0x01xx..0x01F4, bound check 0x01F4). Init at 0x66b5: pointer = 0x01F4.
- fcn.000084e2: address = 0x05F4 + col*36 (B=0x24) with 6-row inner loop;
  second base 0x05F6. Grid = 21 columns x 6 rows = 126 cells.
- fcn.0000b35e executor: 21 outer (limit 0x15) x 6 inner (limit 0x06) = 126-byte
  data-flash transfer per op, one byte per cell via fcn.000084e2.
- 126 bytes is NOT 81x3=243: the saved per-key config is a reduced/compressed
  form of the full 81-key table. (81-key live table = the working color path.)

## Why the F11-template probe wedged the lighting (root cause)

cmd 0x0a writes the 50-byte data-flash config region (0x00-0x32) from the
frame, plus the 126-byte per-key region. The OpenRGB F11 template (141 bytes)
overflowed the 50-byte config with F11-layout fields (mode at the F11 offset),
zeroed the per-key table -> all LEDs off. Fn+Esc factory reset restored.
K75 config layout differs from F11: mode/brightness/speed offsets are in the
50-byte region at the K75's own offsets (default profile: CODE 0xA418, mode
0x35 at +0x0E; save template CODE 0xAD7A -> XDATA 0x0DB2).

## Report-0x09 frame buffer (2026-09-24)

The 519-byte report-0x09 payload lives at XDATA 0x1150+:
- [0x1150] = frame[0] = COMMAND byte (fcn.00008765 dispatch: 0x04 read,
  0x06 save, 0x09 ISP [checks frame[3]==0x05 && frame[4]==0x75 -> fcn.0000ecc4]).
- [0x1155] = frame[5] — read by the 0x66 effect processor with `anl #0x07`
  (effect flags contribution) and used as a response byte by fcn.0000bbcd
  (cmd 0xAA writes [0x1133] -> 0x1155, i.e. writes into frame[5]).
- 0x1130 (the 20-byte command block) is BELOW the frame buffer — a separate
  workspace filled by the copy engine, consumed by fcn.0000b684 when
  [0x0F7F]==1: [0x1130]==0x5A -> fcn.0000bbcd register block
  ([0x1131] 0xAC -> [0x1132] -> 0x0F3F effect index; 0xAA -> [0x1133] -> 0x1155);
  [0x1130]==0x13 -> fcn.0000d2f6 apply gate (fcn.0000218b).

Polymorphic helpers: fcn.0000289d reads one byte — r3==1: XDATA [dptr];
r3==0xFE: IRAM @r1; else: CODE [dptr]. fcn.0000295c advances the pointer at
[dptr]. fcn.00002972 = effect-engine gate (r3:r2:r1 vs r7:r6:r5 compare).

## Frame-pointer convention (2026-09-24)

Every flash-op handler (fcn.00008402 read, fcn.00009308 write, fcn.00007393
serialize, 0x8fb9/0x94ef/0x958f/0x99de/0x9a78) begins with the same idiom:
  [0x0EEE] = r6, [0x0EEF] = r7, [0x0EF0] = r5   (frame pointer pair + offset)
so the command dispatcher passes the FRAME LOCATION in r6:r7 and an offset in
r5. The earlier "frame = 8:0xFA" was the value from one call path only.

cmd 0x04/0x06 share path 0x8834 -> fcn.0000ecb8 (ISP-adjacent handler).
fcn.00008838: bounds check then CODE table 0x0786 + index walk.

## Read-path verdict (2026-09-24, live-probed)

cmd 0x04 (op 0x52) with the activate marker payload[9]=1: GET 0x09 returns
only the 8-byte ACK echo (09 04 00 00 00 00 00 00) — no 0x13 response marker,
no data; interrupt-IN silent. The HID descriptor's report-0x09 input is only
7 bytes. CONCLUSION: cmd 0x04 is an INTERNAL config reload (data-flash ->
live per-key table via the 0x05F4-grid engine); there is NO host-visible
config read on any channel. Config region mapping must come from controlled
write tests (see analysis/config-layout-hypothesis.md).

Also confirmed: cmd 0x0a routes to fcn.00005001 at 0x87bf (shared with the
report-0x06 'S'/'R' protocol); with an RGB-filled payload the 'S'/'R' branches
no-op, so the color effect comes from the cmd-0x0b apply reading the staged
payload, not from fcn.00005001.

## Direct-LED command (cmd 0x08) — decoded (2026-09-24)

fcn.00007108 (instruction-verified):
- entry: r6:r7 = frame pointer, r5 = payload offset where RGB data starts
- loop 6 iterations: reads 3 bytes at frame[r5 + i*3] and writes them to
  XDATA 0x0379 + i*3 (targets 0x0379/0x037A/0x037B... = 18 bytes total)
- consumer: effect engine at 0x2dd6 reads the 0x0379 zone with stride 0x12
  (fcn.00002a0a), status flag byte at 0x0375 ([0x0375] & 7 at 0x311a)

So cmd 0x08 = direct-LED write for a 6-key RGB zone (18 bytes), NOT the
81-key grid. The RK M75/Kreo family header (09 08 00 00 01 00 <len> <RGB...>)
is misaligned for the K75 — that's why the family-format test failed. This
6-key zone is a strong candidate for the side/case underglow strip.
