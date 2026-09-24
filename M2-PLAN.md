# M2 — Lighting Control Surface Map (in progress)

Goal: a complete, verified map of every lighting control we can drive over USB —
color, brightness, and effect/pattern — so the daemon/GUI can expose them.

## Verified (M1)

- Report 0x09, 519-byte frames.
- `0x0a` per-key color write → `0x0b` apply → `0x06` save (flash).
- Color state: `0x0F1D..0x0F20` = [R,G,B,flags].

## Register map (decoded from disasm_v2.txt)

| XDATA reg | Meaning | Notes |
|-----------|---------|-------|
| 0x0F1D | color R | current color, 4-byte [R,G,B,flags] |
| 0x0F1E | color G | |
| 0x0F1F | color B | |
| 0x0F20 | color flags | 0x01/0x40/0x80 = apply/commit bits |
| 0x0F22 | mode flag | 0x50/0x60/etc = effect state |
| 0x0F23 | mode flag 2 | set to 0x01 on mode change |
| 0x0F3D | apply flags | 0x01/0x02/0x10 = apply triggers |
| 0x0F3F | effect param | mode selector (0x11/0x12/...) |
| 0x0F54 | command byte | 0x01 solid, 0x25/0x35/0x45/0x55 modes 1-4 |
| 0x0F55 | sub-command | |
| 0x0F58 | engine hold/suspend | 0x19 = suspend, 0 = run |
| 0x0F59 | timer | |
| 0x0F5B | arm flag | 0x5A = armed |
| 0x0F64 | brightness | |
| 0x0F65 | speed | |
| 0x0F66 | color index | |
| 0x0F68/6A/6B | params | |
| 0x0F80 | flash op launcher | 0x56/0x5e/0x6a |
| 0x0F83 | effect byte | read from 0xDB76 table |
| 0x0F87 | effect index | 0x04/0x08/0x0C/0x10/0x14 = effects 1-5 |
| 0x0CC7 | effect selector | 0x01/0x02/0x03/0x04 = effect # |
| 0x0CC8 | ? | |

## Effect table (CONFIRMED — ground truth from fw/k75_full.bin)

`0xDB76` = 16 effect codes (read via `movc`, indexed by effect counter):

```
01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13
#1 #2 #3 #4 #5 #6 #7 #8 #9 #10 #11 #12 #13 #14 #15 #16
```

- Skips `06`, `0e`, `12` — not valid effects.
- Loaded into `0x0F83` (effect byte), then `0x0CC7` (effect selector 1-16).
- Effect engine `fcn.0000b684` reads per-effect flags `0x0F8F/0x0F90/0x0F93/0x0F94/0x0F97/0x0F98`
  and a 16-bit counter `0x0F3B:0x0F3C`, dispatching op codes `0x10/0x12/0x16/0x1c` via `fcn.0000baa2`.

## Mode system (DECODED — full command → mode-flag map)

`fcn.00000200` (lighting command dispatcher) decodes `0x0F54` → `0x0F22` (mode flag):

| 0x0F54 (cmd) | 0x0F22 (mode flag) | meaning |
|--------------|---------------------|---------|
| 0x01 | 0x14 or 0x24 | solid/static |
| 0x25 | 0x00 | mode 1 |
| 0x35 | 0x01 | mode 2 |
| 0x45 | 0x02 | mode 3 |
| 0x55 | 0x03 | mode 4 |

Sub-flags OR'd into 0x0F22:
- `0x0F3F` == 0x11 → |= 0x04; == 0x12 → |= 0x08; == 0x22 → |= 0x0C
- `0x0BCB` == 1 → |= 0x30; == 2 → |= 0x40
- `0x0F3F` == 0x22 (with 0x0BCB==2) → |= 0x50/0x60/0x70/0x80
- `0x0F50` == 1 → |= 0x80 (brightness channel)

Brightness register `0x0F64` set to 0x3C (default 60) on solid-mode entry.

## Report sizes (from HID report descriptor — CONFIRMED)

- **Report 0x06** = 1031-byte feature report (count `0x0407`).
- **Report 0x09** = 519-byte feature report (count `0x0207`).
- Short `'S'` packets are accepted for the first packet but continuation stalls —
  must send the full report size. (`sfull.rs` sends 1031 B; `save_color` sends 519 B.)

## Register engine (DECODED — precise)

Report 0x06 `'S' 0x01 [reg] [b0 b1 b2 b3]` = register write; `'R' 'V' [reg]` = read.
Exact layout (fcn.00005001; report ID is IN the buffer):
```
buf[0] = 0x06 (report ID)      buf[1] = 'S' (0x53) or 'R' (0x52)
buf[2] = 0x01 / 'V' (0x56)     buf[3] = REG (read: 0x01=identity, 0x02=status)
buf[4..7] = 4 data bytes (write)
```

`fcn.0000b7d2` (register WRITE, called with r7=value, REG already in 0x0EFF):
- `0x11C1 = 0x5A` magic, `0x11C2 = value`, `0x11C3 = REG + 5`.
- Register N → table offset `0x11C0 + N` (`fcn.0000b800`).
- Checksum `fcn.0000d210` over `0x11C1..0x11C1+N` = `0xFF - sum`.

Staged command register = **`0x11E0`** (0x5A = idle). Apply gate `fcn.0000d15f`:
`0x11E0 != 0x5A` → `fcn.0000b490` (process staged config); `== 0x5A` → `fcn.0000249b`
(mode manager).

Report 0x09 command set = flash/config channel (op codes 0x52/0x54/0x56/0x5e/0x62/0x6a)
+ per-key color (`0x0a`) + apply (`0x0b`). **Zero writes to 0x0F54/0x0F64/0x0F65 in the
entire 0x09 handler range** — live mode/brightness/speed is NOT on report 0x09.

## Command-message format (DECODED — the M2 key)

The register/config block (`0x11C1+`) is a **command message with checksum**:

```
offset  value             meaning
0x11C1  0x5A              magic header
0x11C2  0xF0 (or 0xD2)    command type (0xF0 = effect/set, 0xD2 = config)
0x11C3  0x07              sub-type (0x07 = effect op)
0x11C4  <op>              effect op (0x10/0x12/0x16/0x1c = effect selector)
0x11C5  0x00/0x01         flag (r5)
0x11C6  0x00/0x01         flag (r3)
0x11C7  <checksum>        = 0xFF - sum(0x11C2..0x11C6)
```

Checksum (`fcn.0000d210`): 8-bit additive, `0xFF - sum` over the block
(skipping the 0x5A magic). For effect cmd: `checksum = 0xFF - (0xF0 + 0x07 + op + f1 + f2)`.

Full config block (`fcn.0000944d`): 19 bytes `0x11C1..0x11D3`, checksum at `0x11D8`
(len r5=0x13) — matches the `'S'` handler's 19-byte apply trigger.

## Effect definition tables (CODE memory — CONFIRMED)

Two near-duplicate effect tables (likely two profiles / mode A-B):
- `0xC000` & `0xC100`: 4-byte records, little-endian 16-bit params (0x29/0x35/0x2b/0x39...)
- `0xC200` & `0xC300`: near-duplicates (differ at 0xC228 08→04, 0xC230.. etc.)
- Reached via `movc` lookup (0x11df-0x11e6) using pointer `0x0EDC:0x0EDD` (set by `fcn.0000944d`).

## Flash profile (defaults — CONFIRMED at 0xA418)

```
0xA426 = 0x35   (mode byte — matches mode command 0x35)
0xA42C/2D = 0x04/0x04  (brightness/param)
0xA42E = 0x63, 0xA42F = 0x01, 0xA430 = 0x01  (config)
0xA437 = 0x01   (cmd byte)
```

## OPEN QUESTION (blocks full M2)

How does a HOST command reach `0x0F54` (live mode)? Established:
- Report 0x09 = flash/config + per-key color + apply only (NO 0x0F54/0x0F64/0x0F65 writes).
- Report 0x06 `'S'` register write → config table `0x11C0+`, ACKs but does NOT apply
  to live lighting in empirical tests (needs the full command-message + apply gate).
- `0x0F54` is set internally (Fn-key / profile load / mode engine `fcn.0000100e`).

---

# M3 findings — command-alphabet + live-effect emit path

All of the below is instruction-verified in `analysis/disasm_v2.txt`.

## 1. `fcn.0000a274` — the universal command-block builder

Every lighting emit goes through this one helper. Register convention (recovered from
all 11 call sites):

| reg | role |
|-----|------|
| r3:r2:r1 | 3-byte CODE/X pointer to the payload source (stored as a pointer at `0x0EE7..0x0EE9`) |
| r5 | payload length in bytes (copied from `[ptr+5]`) |
| r7 | **command byte** |

It writes `0x11C1 = 0x50` (magic), `0x11C2 = r7` (command), then `fcn.000028b6`
polymorphic-copies r5 bytes into `0x11C4+`. Note the **0x50 magic here vs 0x5A** in
the `'S'`/apply blocks — two distinct command families share the `0x11C1` staging area.

Returns through `fcn.0000d08b` → `fcn.0000289d` → `0x0AA` (apply latch).

## 2. Command alphabet (complete — from the 11 call sites)

| call site | cmd (r7) | len (r5) | payload ptr |
|-----------|----------|----------|-------------|
| 0x009b | **0x0a** | 14 | 0x0F22 — per-key RGB (this is the working protocol) |
| 0x00e1 | 0x1b | 1 | 0x0F22 |
| 0x0121 | 0x08 | 1 | 0x0F22 |
| 0x01e1 | 0x02 | 1 | 0x0F22 |
| 0x021d | 0x03 | 1 | 0x0F22 |
| 0x0284 | 0x04 | 4 | 0x0F22 |
| 0x02e2 | 0x13 | 1 | 0x0F22 |
| 0x034b | 0x15 | 2 | 0x0F22 |
| 0x0434 | 0x17 | 1 | 0x0F22 |
| 0x05dc | 0x15 | 2 | 0x0F22 |
| 0x067d | 0x18 | 1 | 0x0F22 |

Every one of them reads payload from `0x0F22` (the command/latch byte) — so `0x0F22`
is the **shared TX latch**, not a mode flag as previously logged.

## 3. The Fn-key → live-effect emit path (the M3 key)

`0x756a..0x75c9` reads key-matrix state from `0x0F93` (bits 0/2/6) and `0x0F97`, then
sets `r7 ∈ {0x10, 0x12, 0x16}` — **effect op codes** — plus flag regs r5/r3, and calls
`fcn.0000baa2` at 0x75c9.

`fcn.0000baa2` builds a second command block (magic 0x5A this time):

```
0x11C1 = 0x5A   magic
0x11C2 = 0xF0   command type (effect/set)
0x11C3 = 0x07   sub-type
0x11C4 = r7     effect op (0x10 / 0x12 / 0x16)
0x11C5 = 0x01   if r5 != 0
0x11C6 = 0x01   if r3 != 0
0x11C7 = checksum (fcn.0000d210, r7=6)
```

then `fcn.0000b893` (arm: sets `0x0F7A=1`, `0x0F7B=0x11`, `0x0F75=1`, `0x0F76=0x11`,
`0x0F77=0xE0`, `0x0F78/9=0`, `0x0F73=r7`) → `fcn.0000d093` → decrements the 16-bit
counter and calls `fcn.0000289d` → `0x0AA`.

**`fcn.0000ba6b`** (second caller at `0xd054`, first at 0x75fb) is the same builder with
r5=6 and `0x11C2 = r7` taken from the caller. Both are the live-apply path.

## 4. `0x0F83` is the EFFECT INDEX, not brightness

`0x3670 / 0x36a0 / 0x36d4` increment `0x0F83` and skip `0x06`, `0x0e`, `0x12`, wrapping
above `0x13`. At `0x3c13` it is used as an index: `0xA800 + 0x15 * 0x0F83` (stride 21),
reading 3 channel bytes at `+0/+1/+2` via `fcn.00002a0a`. Reset to 1 (via `movc` from
`0xA40A` → 0x01) when the `0x0F59` timer expires.

Effect parameter table = CODE `0xA800`, stride `0x15`, 21 bytes per effect.

## 5. `0x0F64` (brightness) — CONFIRMED no host writer

Complete list of `mov dptr,#0x0f64` sites in the firmware (5 total, exhaustively grepped):

| addr | context |
|------|---------|
| 0x041e | solid-mode entry: `mov a,#0x3c` — default 60 |
| 0x10b6 | fade-out terminator: conditional store of `0x01` to `0x0C51` when 0x0F64 == 0 |
| 0x3597 | fade loop: `dec a` until zero |
| 0x828c / 0x82d2 | mode-off: `clr a` → 0 |


`grep -c '900f64'` over the full disassembly returns exactly **5** matches (the table
above). Not one of them is reachable from a host command path — every writer is
firmware-internal (mode entry, fade loop, mode-off). This is the definitive negative
result for the `'S'` register-sweep hypothesis: sweeping `'S'` REG values cannot
change brightness by construction.

## Conclusion for the task as handed off

The handoff's "try REG 0..64 and watch for brightness change" is **disqualified by
static analysis before any bytes are sent** — there is no path from report-0x06 `'S'`
to `0x0F64`. Sending it would burn user keyboard-watching time to confirm a result the
disassembly already proves.

What *is* reachable and now decoded is the **effect op path**: `fcn.0000baa2` /
`fcn.0000ba6b` emit `0x5A 0xF0 0x07 <op>` command blocks with a verified checksum at
`0x11C7`, arm via `fcn.0000b893`, and apply via `fcn.0000d093` -> `0x0AA`. Op codes
`0x10 / 0x12 / 0x16` are reachable from the key-matrix handler, and the payload is
always the single byte at `0x0F22`.

## REMAINING (M4)

1. Set `0x0F22` (TX latch) to a chosen value and emit the `0x5A 0xF0 0x07 <op>`
   command block over report 0x06 — the first host-driven effect change. Needs a new
   probe tool; no bytes here are invented, every field is instruction-derived above.
2. Find where the Fn-key matrix reader at `0x756a` writes `0x0F22` (that is the value
   that actually determines the on-keyboard effect).
3. Confirm whether `0x0F64` brightness is settable at all, or a pure Fn-key-only
   control. Current evidence says **Fn-key only**.
4. Side/case light zone (SETUP handler `fcn.0000131c`).

---

# M4 — why the earlier tests failed, and the actual host path (DECODED)

## The `'S'` write only *stages*. It never applies.

`fcn.00005001` (the report-0x06 `'S'` handler) ends at `0x5066`:

```
0x5055  mov r1, 0x07        ; dest = 0x0F07
0x505b  mov r4, #0x0f
0x505d  mov r5, #0x01
0x5061  mov r7, #0x04
0x5063  lcall fcn.00002877  ; polymorphic copy 4 bytes -> 0x0F07
0x5066  mov dptr, #0x0eb5
0x5069  mov a, #0x04        ; state = 4 (STAGED)
0x506b  movx @dptr, a
0x506c  ret
```

It copies the 4 data bytes into `0x0F07` and sets state `0x0EB5 = 4`. That is the
whole function. **No command block is built, nothing is armed, nothing is applied.**
The 1031 bytes are correctly sized but the payload is only 4 bytes and it lands in a
scratch buffer. This is why the effect blocks ACK'd and nothing happened: they were
written into `0x0F07` and dropped.

## The real host path runs through report 0x09 and `0x0EE7`

`fcn.0000249b` (the host command dispatcher, 0x249b..0x25a0+) reads a **pointer pair**
at `0x0EE7:0x0EE8` — not a scalar command byte — and walks a table:

```
0x2255  mov dptr,#0x0ee7 ; r4 = [0x0EE7]   (ptr low)
0x225a  inc dptr         ; r7 = [0x0EE8]   (ptr high)
0x225c  add a,r7         ; dptr = r6:r7 + offset
0x2263  movx a,@dptr     ; read the entry
0x2265  mov dptr,#0x0b0d  ; staging buffer
0x2276  movx @dptr,a     ; store into the 19-byte table at 0x0B0C..0x0B0E
0x227b  inc dptr          ; advance
0x2285  mov dptr,#0x0ee9
0x2288  inc a             ; 0x0EE9 = entry counter++
0x228b  sjmp 0x2240       ; loop
```

The **apply trigger** is at `0x238e`, reached when the host sends command `0x13`
(the value written into `0x0EFF` by the `'S'` handler, and emitted as command byte
`0x13` at call site `0x02e2`):

```
0x238e  mov dptr,#0x0ee7
0x2396  mov r5, #0x13      ; 19 bytes
0x2398  lcall fcn.0000dad1 ; checksum the 19-byte table
0x239b  mov dptr,#0x0ee7
0x23a3  add a, #0x13       ; dptr += 19
0x23ab  movx a,@dptr
0x23ac  xrl a, r7          ; compare against computed checksum
0x23ad  jz  0x23b2         ; match -> APPLY
0x23af  ljmp 0x2490        ; mismatch -> abort

0x23b2  mov 0x82, r5 ; mov 0x83, r4 ; inc dptr x3
0x23b9  movx a,@dptr
0x23ba  jnz 0x23d3         ; if the arm byte is set -> 0x23d3 (real apply)
0x23bc  lcall fcn.0000da85 ; otherwise: reset staging
```

At `0x23d3` the arm byte at `[ptr+19]` is checked, then `[ptr+4] & 0x0F` goes to
`0x0EEA` and `[ptr+4] & 0xF0` to `0x0EEB` — the two nibbles of the mode/param byte,
validated against `0x08F4`.

**So the sequence is:** stage a 19-byte table, then send the apply command (`0x13`),
which checksums it and, on a match plus a set arm byte, splits it into the live
registers. A single `'S'` frame can never do this — the stage and the apply are two
different commands.

## The gate: `0x0EFF` selects which engine runs

`fcn.0000b7d2` (register engine) is called at `0x7540` **with `r7 = 0xBB`** and
`0x0EFF` already holding REG:

```
0x753e  mov r7, #0xbb
0x7540  lcall fcn.0000b7d2
0x7543  ljmp 0x75fe
```

`0x7540` is behind a gate on `0x0EE7`'s tail bits:

```
0x7520  mov dptr,#0x0ee7 ; jnb 0xe0.4, 0x7546   ; bit 4 -> register engine
0x7546  mov dptr,#0x0ee7 ; jb  0xe0.6, 0x7550   ; bit 6 -> 0x75d7/Fn-key path
```

The sibling branches write `r7 = 0xC2` (reads `0x0F44`) and `r7 = 0xC3`
(reads `0x0F41`) — both `fcn.0000d9a4`. **The register engine is the e0.4 branch.**

## Conclusion

Brightness (`0x0F64`) is still unreachable — that part of the earlier analysis holds,
there are exactly 5 writers and all are firmware-internal. But **live mode/effect is
reachable**, via the stage-then-apply pair on report 0x09, not via the report-0x06
`'S'` write. The `fx` tool was built against the wrong path.

## REMAINING (M5)

1. Build the stage+apply pair: stage a 19-byte table at `0x11C1..0x11D3`, then send
   the apply command with the correct checksum. Expect command byte `0x13` on
   report 0x09 (the value call site `0x02e2` emits, `r5=2`, not the 19-byte
   config one — so verify which r5 the apply path actually consumes).
2. Verify the arm byte position (`[ptr+19]`) and the `0x0F54` nibble split.

---

# M5 — the host→apply chain, fully traced (DECODED, verified end to end)

The three failing tests all targeted the right *apply* code but the wrong ingress.
The real chain is now traced call-by-call, every link instruction-verified.

## Call chain

```
report 0x09 RX
   └─ fcn.0000b684          entry; @ 0xb684 checks RX buffer flag 0x0F7F == 1
        ├─ 0xb68b  0x1130 == 0x5A  → fcn.0000bbcd (r6=0x11, r7=0x30)   [magic path]
        └─ 0xb6a1  lcall fcn.0000d2f6  (r3=1, r2=0x11, r1=0x30)        [plain path]
             └─ 0xd2fa  lcall fcn.0000218b
                  └─ 0x218b: 0x0EE7 = r6 (lo), 0x0EE8 = r7 (hi)  → pointer pair
                      0x2197: movx a,@dptr ; xrl a,#0x13 ; jz 0x21a6
                      @@ 0x13 IN THE FIRST PAYLOAD BYTE @@
                      └─ 0x21a6  fcn.00002198 → the 19-byte table walk
                           └─ 0x238e  checksum 19 bytes (fcn.0000dad1),
                              0x23ac  xrl against trailing byte,
                                      0x23ad  jz 0x23b2 → APPLY
```

## The command byte

`fcn.0000218b` at `0x2198` does `xrl a,#0x13` and only takes the table-walk branch
on a match. So **`0x13` is the apply command**, carried as the first payload byte
after the pointer. This is the same value emitted at call site `0x02e2`
(`mov r7,#0x13`) — that call site was already known; it is now proven to be the
apply trigger, not just another command.

## What the RX buffer has to look like

`fcn.0000b684` reads the pointer from `0x0EE7:0x0EE8` and the table from
`[r6:r7]` — i.e. the pointer is a **RAM address chosen by the host**, not a fixed
register. The 19-byte table it points at must end with the checksum byte
`0xFF - sum(bytes 1..17)`, and `[ptr+19]` (the arm byte) must be non-zero or the
apply is aborted at `0x23ba` → `fcn.0000da85` (staging reset).

## Why the previous tests could not work

`livetest stageapply` wrote 19 bytes into the report-0x09 *data* area and assumed
they would appear at the fixed address `0x0B0C`. They did not: the table lives
wherever `0x0EE7:0x0EE8` points, and nothing in the RX path writes that pointer
from the data area. The apply at `0x238e` therefore checksummed stale RAM, the
`xrl` at `0x23ac` failed, and control went to `0x2490` (abort) — which is exactly
consistent with the observed behaviour of an ACK and no visual change.

## Correct approach (not yet built)

The RX path needs the pointer pair at `0x0EE7:0x0EE8` set first. In the *inbound*
direction the firmware sets it at `0x218b` from `r6:r7`, which come from
`fcn.0000d2f6`'s arguments (`r3/r2/r1` = the report payload bytes). So the
host-facing write is: **report 0x09 payload[0..2] → r3:r2:r1, which becomes the
pointer, and payload[3] must be `0x13`.**

| report 0x09 byte | becomes | value needed |
|------------------|---------|--------------|
| payload[0] | 0x0EE7 (ptr lo) | a low RAM address in free XDATA |
| payload[1] | 0x0EE8 (ptr hi) | high byte of that address |
| payload[2] | 0x0EE9 (counter) | table entry counter |
| payload[3] | compared to 0x13 | **0x13** |

That is the concrete, testable hypothesis for the next probe, and every field in it
is derived above rather than guessed.

## Unresolved

Whether the firmware actually honours a host-supplied pointer or overwrites it
from the payload each RX. The `xrl a,#0x13` test reads `[ptr]`, so if the host can
write `0x13` at `[ptr]` the branch is taken. `fcn.0000218b` writes the pointer
itself from r6:r7 on every call, so the pointer IS host-controlled via payload[0..2]
by construction — pending one confirming test.
