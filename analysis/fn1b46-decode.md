# K75 Firmware — fcn.00001b46 & fcn.000057db Decode + Report-0x09 Frame→State Map

> **Historical analysis; do not use this document to build USB packets.**
> Its sections contradict each other and the subsequently traced receive path.
> Claims that XDATA `0x1100..0x1303` is a contiguous 519-byte report, that
> `0x1150` is any host payload byte, or that XDATA addresses are report offsets
> are wrong. `0x1150` is transfer state, and `0x7253` copies successive
> eight-byte USB chunks from `0x1100` to `0x08FA`. The **VERIFIED** labels below
> are historical labels and do not establish host packet layout. Use
> [the current ingress audit](../docs/protocol-audit.md) instead.

Static analysis of the SH68F90A disassembly (`disasm_v2.txt`). Addresses = file offsets. Only instruction-derived claims are made; each claim is tagged **VERIFIED** (trace complete), **INFERRED** (a link is missing), or **UNKNOWN**.

---

## 0. Frame buffer & dispatch context (established first)

- The 519-byte report-0x09 payload is DMA'd into XDATA **0x1100–0x1303**. The USB IRQ path (see `fcn.00008765` / `fcn.00008838`) reads the **command byte** at **[0x1150]** and branches:
  - `[0x1150]==0x0a` → `lcall fcn.00005001` (config-write fast path) then clears `[0x1150]` (0x87c3–0x87d4). **VERIFIED**
  - `[0x1150]==0x09` → re-reads `[0x1150]`, XORs 0x09, validates magic `[0x1101]==0x05` then `[0x1102]==0x75`, then `ljmp 0xecc4` (0x8790–0x87bc). **VERIFIED**
  - At `0xecc4`: `clr 0xa8.7 ; mov 0xf0,#0xa5 ; mov a,#0x5a ; ljmp 0xff00`. **VERIFIED**
  - `0xff00` checks `a==0x5a` and `0xf0==0xa5` then performs the boot-block flash unlock/erase sequence (`lcall fcn.0000fc5f`, `lcall fcn.0000fd28`, `lcall fcn.0000fda7`) and `ljmp 0xfdee`. **VERIFIED** — this is the boot-block flash-program path; the 519-byte frame body itself is the data being programmed.
- Other dispatch arms (cmd 0x04, 0x06, 0x0b/0x0c/0x0d) at `0x87d5`–`0x881a` set state flags and return; only **0x0a** and the **0x09 magic path** route frame data into application state. **VERIFIED**

**Frame byte offset convention (0-based within the 519-byte payload, base = 0x1100):**
`frame[N] == XDATA[0x1100 + N]`. So `[0x1150]` = frame byte **0x50** (command), `[0x1101]`=byte 1, `[0x1102]`=byte 2, `[0x1130]`=byte 0x30, `[0x11C1]`=byte 0xC1, `[0x1131]`=byte 0x31, `[0x1132]`=byte 0x32, etc.

---

## 1. fcn.00001b46 — the frame/state processor (0x1b46–0x1e6d, + continuation 0x1e6e–0x218a)

Radare boxes the main body as `┌ 808: fcn.00001b46` (line 5924) ending at the `ret` at `0x1e6d`. The bytes 0x1e6e–0x218a are reachable only via `jmp @a+dptr`/`ljmp` from inside the function (XREFs annotated `from fcn.00001b46 @ +0x363(x)` etc.), so they are a logical continuation (a shared jump table tail). **VERIFIED**

### Entry contract
- Entered with **r7 = hi byte** and **r5 = lo byte** of a 16-bit value that the caller wants stored as the new "frame pointer pair" (`[0x0EE7]`/`[0x0EE8]`).
- Prologue (0x1b46–0x1b4d):
  - `mov dptr,#0x0EE7 ; mov a,r7 ; movx @dptr,a ; inc dptr ; mov a,r5 ; movx @dptr,a` → **[0x0EE7] = r7, [0x0EE8] = r5**. **VERIFIED**
- Side-note: `dptr` on entry is irrelevant (overwritten immediately); the contract is solely (r7,r5). **VERIFIED**

### Short-circuit on bit 0x27.5 (0x1b4e–0x1b58)
- `jnb 0x27.5, 0x1b59` — if bit 0x27.5 is set, the function sets `[0x0F3E] |= 0x02` (the "staged dirty" flag) and returns immediately. **VERIFIED** → this is the "discard new pointer, just mark staged-config dirty" fast path.

### Body — gate & frame-index derivation (0x1b59–0x1b8d)
1. `mov dptr,#0x0C45 ; mov a,#0x01 ; movx @dptr,a` → **[0x0C45] = 1** (live-config gate "busy"). **VERIFIED** (0x1b59–0x1b5f)
2. `clr a ; mov dptr,#0x05F3 ; movx @dptr,a` → **[0x05F3] = 0**. **VERIFIED** (0x1b60–0x1b63) — purpose UNKNOWN beyond "scratch clear".
3. `setb 0x28.3`. **VERIFIED** (0x1b64)
4. Frame-index calc (0x1b66–0x1b77):
   - `mov dptr,#0x0EE8 ; movx a,@dptr ; mov 0xf0,#0x06 ; mul ab ; mov r7,a` → r7 = **[0x0EE8] * 6** (low byte). **VERIFIED**
   - `mov dptr,#0x0EE7 ; movx a,@dptr ; add a,r7 ; mov dptr,#0x0EE9 ; movx @dptr,a` → **[0x0EE9] = [0x0EE7] + [0x0EE8]*6**. **VERIFIED** — this is the per-frame "slot index".
5. Reads two gating bytes (0x1b78–0x1b8d):
   - `mov dptr,#0x0BC2 ; movx a,@dptr ; mov r7,a ; mov dptr,#0x0EEA ; movx @dptr,a` → **[0x0EEA] = [0x0BC2]**. **VERIFIED**
   - `mov dptr,#0x0BC1 ; movx a,@dptr ; jnz 0x1b8a ; ljmp 0x1c5f` — if **[0x0BC1]==0** skip to 0x1c5f (the "no active command" tail). **VERIFIED**
   - At 0x1b8a: `mov a,r7 ; jz 0x1b90 ; ljmp 0x1c5f` — if **[0x0BC2]==0** also skip. **VERIFIED**

### Active-command branch (0x1b90–0x1c5e) — reads a per-slot table at 0x0BD4
- 0x1b90: `mov dptr,#0x0F50 ; movx a,@dptr ; mov r7,#0 ; add a,0xE0 ; mov r6,a` → r6 = **[0x0F50] + [0xE0]**. **VERIFIED** ([0xE0] is a fixed page register; [0x0F50] is the lighting-param "channel".)
- 0x1b9c–0x1ba7: `mov 0x82,a ; mov a,#0xD4 ; addc a,r6 ; mov 0x83,a` → builds pointer **0x0BD4 + ([0x0F50]+[0xE0])**. Pushes 0x82/0x83 (the low/high of this pointer). **VERIFIED**
- 0x1ba7–0x1bb3: `mov dptr,#0x0EE9 ; movx a,@dptr ; mov r3,a ; pop 0x82 ; pop 0x83 ; mov 0xf0,#0x04 ; lcall fcn.00002a0a`
  - `fcn.00002a0a` (0x2a0a, 12 bytes) does: `mul ab ; add a,0x82 ; mov 0x82,a ; mov a,0xf0 ; addc a,0x83 ; mov 0x83,a ; ret` — i.e. **dptr += a*0xf0** with a=r3=[0x0EE9] and 0xf0=4. So final pointer = `0x0BD4 + ([0x0F50]+[0xE0]) + [0x0EE9]*4`. **VERIFIED**
- 0x1bb6: `lcall fcn.000029bd` — reads 4 bytes from that table pointer into r4:r5:r6:r7 (`movc a,@a+dptr` ×4). **VERIFIED** → the 4-byte record at `0x0BD4 + offset*4` is loaded.
- 0x1bb9–0x1bbf: `orl a,r4 ; orl a,r5 ; orl a,r6 ; orl a,r7 ; jnz 0x1bc2 ; ljmp 0x1c5f` — if the 4-byte record is all-zero, bail to 0x1c5f. **VERIFIED**

### "Stride" write & sub-dispatch (0x1bc2–0x1c3c)
- 0x1bc2: `mov dptr,#0x0C4F ; movx a,@dptr ; dec a ; mov r7,a` → r7 = **[0x0C4F] - 1**. **VERIFIED** ([0x0C4F] is the per-slot "stride".)
- 0x1bc8–0x1bd5: reads `[0x0EE8]→r6`, `[0x0EE7]→r5`, `mov 0xf0,#0x15 ; mul ab` → r5*r5(low)? Actually `a=r5, 0xf0=0x15, mul ab` gives `a=r5*0x15`. Then `add a,#0xF7 ; mov 0x82,a ; clr a ; addc a,#0x04 ; mov 0x83,a` → pointer base **0x04F7 + r5*0x15**. Then adds r6 (`[0x0EE8]`) → **0x04F7 + [0x0EE7]*0x15 + [0x0EE8]**. **VERIFIED**
- 0x1be9: `mov a,r7 ; movx @dptr,a` → writes `[0x0C4F]-1` to that pointer. **VERIFIED**
- 0x1beb: `jnb 0x29.2, 0x1c3e` — branch on bit 0x29.2.

**Branch A (0x1bee–0x1c3c), bit 0x29.2 set:**
- 0x1bee–0x1bfa: `mov dptr,#0x0EE9 ; movx a,@dptr ; mov r7,a ; add a,#0xD2 ; mov 0x82,a ; clr a ; addc a,#0x0B ; mov 0x83,a` → pointer **0x0BD2 + [0x0EE9]**. **VERIFIED**
- 0x1bfc: `movx a,@dptr ; inc a ; movx @dptr,a` → **[0x0BD2 + [0x0EE9]] += 1**. **VERIFIED**
- 0x1bff–0x1c15: `mov dptr,#0x0BC1 ; movx a,@dptr ; anl a,#0x7F ; mov r4,a ; mov a,#0xD2 ; add a,r7 ; mov 0x82,a ; clr a ; addc a,#0x0B ; mov 0x83,a ; movx a,@dptr ; clr c ; subb a,r4 ; jnc 0x1c18 ; ljmp 0x1e6d` → compares `[0x0BD2 + [0x0EE9]]` (post-increment) against `[0x0BC1] & 0x7F`; if the counter overshoots the limit, **return** (0x1e6d). **VERIFIED** — this is a per-slot repeat-counter with a ceiling read from `[0x0BC1]`.
- 0x1c18–0x1c3c: on counter OK: `clr a ; mov dptr,#0x0EEA ; movx @dptr,a` (**[0x0EEA]=0**); then re-reads `[0x0C4F]→r7`, recomputes the 0x04F7-base pointer using r5=[0x0EE7], r6=[0x0EE8], writes r7 (`[0x0C4F]`) to it; `sjmp 0x1c5f`. **VERIFIED**

**Branch B (0x1c3e–0x1c5e), bit 0x29.2 clear:**
- 0x1c3e–0x1c4b: `mov a,#0xD2 ; add a,r3 ; mov 0x82,a ; clr a ; addc a,#0x0B ; mov 0x83,a ; movx a,@dptr ; jz 0x1c4e ; ljmp 0x1e6d` — reads `[0x0BD2 + r3]` (r3=[0x0EE9]); if non-zero, **return**. **VERIFIED**
- 0x1c4e–0x1c5d: `mov dptr,#0x0EE9 ; movx a,@dptr ; add a,#0xD2 ; mov 0x82,a ; clr a ; addc a,#0x0B ; mov 0x83,a ; mov a,#0x01 ; movx @dptr,a ; ret` → sets **[0x0BD2 + [0x0EE9]] = 1** and returns. **VERIFIED**

### Tail 0x1c5f–0x1c85 — writes the per-frame status byte at 0x0B17
- 0x1c5f–0x1c6d: `mov dptr,#0x0EEA ; movx a,@dptr ; mov r7,a ; mov dptr,#0x0EE8 ; movx a,@dptr ; mov r6,a ; mov dptr,#0x0EE7 ; movx a,@dptr ; mov 0xf0,#0x15 ; mul ab` → r6=[0x0EE8], a=[0x0EE7]*0x15. **VERIFIED**
- 0x1c71–0x1c85: `add a,#0x17 ; mov 0x82,a ; clr a ; addc a,#0x0B ; mov 0x83,a ; mov a,0x82 ; add a,r6 ; ... ; mov a,r7 ; movx @dptr,a` → writes r7 (=[0x0EEA]) to **0x0B17 + [0x0EE7]*0x15 + [0x0EE8]**. **VERIFIED** — a per-(slot,channel) status byte table at base **0x0B17**.

### Mode-gate dispatch on [0x0BA9] (0x1c86–0x1ca0)
- 0x1c86: `mov dptr,#0x0BA9 ; movx a,@dptr ; cjne a,#0x02,0x1c90 ; ljmp 0x1e46` — if **[0x0BA9]==2** jump to 0x1e46 (clears [0x0BA8], ret). **VERIFIED**
- 0x1c90–0x1ca0: else `mov r3,#0x01 ; mov r2,#0x0C ; mov r1,#0x47 ; mov dptr,#0x0EE9 ; movx a,@dptr ; mov r5,a ; mov dptr,#0x0EEF ; mov a,r7 ; movx @dptr,a` (**[0x0EEF]=r7**), then `lcall fcn.00007856`. **VERIFIED**

  - **`fcn.00007856`** (0x7856, 274 bytes): saves (r3,r2,r1) to `[0x0EEB..0x0EED]` and r5 to `[0x0EEE]`; dispatches on `[0x0EEE]` (the saved r7) cases 1/2/3/4 → builds a pointer `0xBC4 / 0xCC4 / 0xD4 / 0xBC + [0x0F50]+[0xE0] + [0x0EEE]*4` via `fcn.00002a0a`+`fcn.000029bd`, reads a 4-byte record, and on `[0x0EEE]==3` additionally calls `fcn.000029a3`/`fcn.00002983`/`fcn.00002972` (the effect-engine gate). In short: **selects a per-mode lighting recipe from one of four tables indexed by [0x0F50]+[0xE0] and the slot index [0x0EEE]**. **VERIFIED** (structurally; the recipe semantics per mode are INFERRED).

### Post-7856 dispatch on [0x0C47] (0x1ca3–0x1cc4)
- 0x1ca3: `mov dptr,#0x0C47 ; movx a,@dptr ; dec a ; jz 0x1cd7 ; dec a ; jz 0x1d1b ; dec a ; jnz 0x1cb3 ; ljmp 0x1d3e` — switch on **[0x0C47]** (values 1,2,3 → 0x1cd7/0x1d1b/0x1d3e). **VERIFIED**
- 0x1cb3–0x1cc4: for [0x0C47] ≥ 4, further sub-switch (`add a,#0xFC` etc.) → 0x1e4c / 0x1e55 / 0x1e5e; default `ljmp 0x1e6d` (return). **VERIFIED**

**Case [0x0C47]==1 (0x1cd7–0x1d1a) — "macro key" handler:**
- Reads `[0x0C4D]→r7`; if 0 or ==0x69, return (0x1cdc–0x1ce7). **VERIFIED**
- Reads `[0x0C48]`; for values 3/4/5 writes 0x04/0x08/0x10 to `@r0` with r0=0x78; default writes `[0x0C48]` raw (0x1cea–0x1d17). **VERIFIED**
- `setb 0x21.4 ; ret` (0x1d18–0x1d1a). **VERIFIED**
  - The `@r0` with r0=0x78 target is IRAM 0x78 — this is the macro-key slot register. **VERIFIED** that the value derives from `[0x0C48]`; **INFERRED** that 0x78 is the macro slot.

**Case [0x0C47]==2 (0x1d1b–0x1d3d) — "media key" handler:**
- Reads `[0x0C4D]→r7`; if 0 or ==0x69 return (0x1d1e–0x1d2b). **VERIFIED**
- `mov dptr,#0x0C4A ; movx a,@dptr ; mov r0,#0x57 ; mov @r0,a` → IRAM 0x57 = **[0x0C4A]** (0x1d2e–0x1d34). **VERIFIED**
- `mov dptr,#0x0C49 ; movx a,@dptr ; inc r0 ; mov @r0,a ; setb 0x21.3 ; ret` → IRAM 0x58 = **[0x0C49]** (0x1d35–0x1d3d). **VERIFIED**

**Case [0x0C47]==3 (0x1d3e–0x1d66) — "config-change" handler:**
- Same `[0x0C4D]` gate (0x1d3e–0x1d4e). **VERIFIED**
- 0x1d51: `mov dptr,#0x0BA8 ; movx a,@dptr ; jz 0x1d66` — if `[0x0BA8]==0` skip. **VERIFIED**
- 0x1d57–0x1d66: `mov dptr,#0x0C4A ; movx a,@dptr ; mov r7,a ; mov dptr,#0x0BAC ; movx a,@dptr ; xrl a,r7 ; jnz 0x1d66 ; lcall fcn.0000D189` — if `[0x0C4A] != [0x0BAC]`, call `fcn.0000D189`. **VERIFIED** (this is a "layer changed" trigger).

**Case [0x0C47] sub-values (0x1d66–0x1e45) — layer/profile reload:**
- 0x1d66: `lcall fcn.0000D189` then sets `[0x0BA8]=1`, copies `[0x0C48]→[0x0BA9]`, `[0x0C49]→[0x0BAA]:[0x0BAB]`, `[0x0C4A]→[0x0BAC]`. **VERIFIED**
- 0x1d8c–0x1e45: long block that loads a 4-byte recipe from a `movc` table at **0xDC??** indexed by `[0x0BAC]*4` and `[0x0BAC]*4+...`, performs 16-bit arithmetic against `[0x0BAE]`, updates `[0x0BAD]:[0x0BAE]`, `[0x0BAF]:[0x0BB0]`, `[0x0BAB]:[0x0BAA]`, `[0x0BB2]:[0x0BB3]`, and calls `fcn.000029cd` (4-byte store to `[0x0F1D..0x0F20]`, the staging buffer). **VERIFIED** structurally; the exact recipe semantics are INFERRED (looks like an animation-tween setup).

**Tail returns / sub-cases:**
- 0x1e46–0x1e4b: `clr a ; mov dptr,#0x0BA8 ; movx @dptr,a ; ret` — clear [0x0BA8]. **VERIFIED**
- 0x1e4c: `ljmp fcn.00000F45` (with r3=1,r2=0x0C,r1=0x47). **VERIFIED**
- 0x1e55: `ljmp fcn.00004810` (same regs). **VERIFIED**
- 0x1e5e–0x1e6c: reads `[0x0C48]`; if 0 writes `[0x0BC2]=1`, else writes `[0x0BC2]=2`; ret. **VERIFIED**
- 0x1e6d: `ret` (the common exit). **VERIFIED**

### Continuation 0x1e6e–0x218a — reached via `jmp @a+dptr` from 0x1e99 (a second-tier dispatcher)
- 0x1e6e–0x1e99: reads `[0x0003]` via `fcn.000028b6` (r3=1 path = XDATA read), adds offsets, does a `cjne a,#0x0E` and `jc 0x1e8c`, then `mov dptr,#0x1E9A ; mov 0xf0,#3 ; mul ab ; ... ; jmp @a+dptr` — a 14-entry jump table at **0x1E9A** (each entry is a 3-byte `ljmp`). **VERIFIED**
- Table targets (0x1e9a–0x1ec1): mostly `ljmp 0x2100` (5 entries), `ljmp 0x218a`, `ljmp 0x1ec4`, `ljmp 0x1edc` ×2, `ljmp 0x1ee4`, `ljmp 0x20fd` ×2. **VERIFIED**

**Sub-handler 0x1ec4–0x1edb** (reached for one table entry):
- `clr 0x27.0` ; reads `[0x0C4E]` ; if `[0x0C4E] >= 0xC8` then `[0x0F3E] |= 0x02`; clears `[0x0C4E]`; ret. **VERIFIED** — `[0x0C4E]` is a debounce/hold counter; threshold 200 (0xC8).

**Sub-handler 0x1edc–0x1ee3:**
- `clr 0x29.0 ; clr a ; mov dptr,#0x0F5F ; movx @dptr,a ; ret` → clears [0x0F5F]. **VERIFIED**

**Sub-handler 0x1ee4–0x20f2 — the big "lighting param update" path:**
- Gates on `[0x0C4D]` (must be non-zero and !=0x69), `[0x0F58]` (must be 0), reads `[0x0C48]→[0x0BA9]`, then dispatches on `[0x0CC6]`:
  - `[0x0CC6]` 4–9 with `[0x27.3]` set → clears `[0x27.3]`, reads `[0x0F1D..0x0F20]` into r4:r5:r6:r7, sets r6 |= 0x20, writes back via `fcn.000029cd`. **VERIFIED**
  - `[0x0CC6]==0x0B` → similar with r6 |= 0x40. **VERIFIED**
  - `[0x0CC6]==2 or 3` → reads `[0x0F83]`, dispatches on it (values 0/3/0x0F/0x10/0x13 for case 6; value 7 with `[0x0F83]==0`; case 8 with `[0x0F83]` in {0,1,0x13}). For matching cases clears `0x26.1`. **VERIFIED** structurally.
  - Default (0x1fd5): reads `[0x0F1D..0x0F20]`, r6 |= 0x20, writes back, `setb 0x27.3`. **VERIFIED**
- 0x2016 onwards: second-tier switch on `[0x0CC6]-0xFB` (i.e. values 4,5,6,7,8...) → 0x2064/0x2090/0x209f/0x20e2/0x20e8/etc. Many of these manipulate `[0x0BC6]`, `[0x0CC7]`, `[0x0F83]`, `[0x08E8]`. **VERIFIED** for control flow; per-case semantics INFERRED (per-mode lighting re-arm).

**Sub-handler 0x2100–0x215e — effect-index update (the 0x0F3F write):**
- 0x2100: `jb 0x26.6, 0x2106 ; ljmp 0x218a` — gate on bit 0x26.6. **VERIFIED**
- 0x2106–0x210d: clears `[0x0F52]:[0x0F53]`, clears `0x26.6`. **VERIFIED**
- **0x210f–0x2114: `mov dptr,#0x0F3F ; mov a,#0x12 ; movx @dptr,a`** → **[0x0F3F] = 0x12 (constant 18)**. **VERIFIED** — this is a hard-coded effect index, NOT frame-derived.
- 0x2115–0x2122: `[0x0BCB]=2 ; [0x0F3D] |= 0x01 ; setb 0x26.2`. **VERIFIED**
- 0x2124–0x2138: reads `[0x0F1D..0x0F20]`, r7 |= 0x02, writes back via `fcn.000029cd`. **VERIFIED**
- 0x213b–0x2153: reads `[0x0F83]`; if it is 4, 7, 9, or 0x0C falls through to 0x215a, else `ljmp 0x218a`. **VERIFIED**
- 0x215a–0x215e: `setb 0x23.6 ; setb 0x2a.7 ; ret`. **VERIFIED**

**Sub-handlers 0x2155, 0x215f, 0x218a** — short bit-clear tails. **VERIFIED**

### Bank-switching
- No `mov PSW` / `0xD0 PSW` write appears anywhere in 0x1b46–0x218a. **VERIFIED** — register bank 0 is used throughout; the `push 0xD0 / mov 0xD0,#0 / pop 0xD0` sequence at 0xa750 is in `fcn.0000a6de`'s IRQ wrapper, not here.

---

## 2. fcn.000057db — the key-matrix scan walker (0x57db–0x596a)

### Entry contract
- 0x57db: `mov 0x1d, r7` ; 0x57dd: `mov 0x1e, r5` → saves **r7→IRAM[0x1D] (row index 0–5)**, **r5→IRAM[0x1E] (column-byte index 0–15)**. **VERIFIED**
- 0x57df: `jnb 0x26.7, 0x57e5 ; ljmp 0x596a` — if bit 0x26.7 set, return early. **VERIFIED**
- 0x57e5: `jb 0x2b.4, 0x57eb ; ljmp 0x5897` — bit 0x2b.4 (key-pressed latch) gates the two main arms. **VERIFIED**

### Arm A (bit 0x2b.4 set, 0x57eb–0x5853) — key-down processing
- 0x57eb–0x57fe: `a=[0x1D] ; 0xf0=0x15 ; mul ab ; add a,#0xF7 ; mov 0x82,a ; clr a ; addc a,#0x04 ; mov 0x83,a ; mov a,0x82 ; add a,0x1E ; mov 0x82,a ; ...` → builds pointer **0x04F7 + row*0x15 + col**. **VERIFIED** — this is the **key-state table at XDATA 0x04F7** (one 0x15-byte record per row, indexed by column within the row).
- 0x5805: `movx a,@dptr ; inc a ; movx @dptr,a` → **[0x04F7 + row*0x15 + col] += 1** (a debounce/hold counter). **VERIFIED**
- 0x5808–0x5820: recompute the same 0x04F7-base pointer, `movx a,@dptr ; mov r7,a` → r7 = the (incremented) counter. **VERIFIED**
- 0x5824–0x582a: `mov dptr,#0x0C4F ; movx a,@dptr ; mov r6,a ; mov a,r7 ; cjne a,0x06, 0x5856` → reads **[0x0C4F] (stride) into r6**, compares r7 (counter) against **r6** (which holds the contents of IRAM[0x06] — a global). If equal, take the "key fully pressed" branch at 0x582d; else jump to 0x5856. **VERIFIED**
  - **`[0x0C4F]` semantics**: the "stride" / threshold — the count at which a key is considered fully actuated. **INFERRED** from usage (it's the value the per-key counter is compared against).
- 0x582d–0x5849 (counter == stride): recompute pointer **0x0575 + row*0x15 + col** (`a=[0x1D] ; 0xf0=0x15 ; mul ab ; add a,#0x75 ; mov 0x82,a ; clr a ; addc a,#0x05 ; ...; add a,0x1E ; ...`), then `mov a,#0x01 ; movx @dptr,a` → **[0x0575 + row*0x15 + col] = 1**. **VERIFIED**
  - **The table at XDATA 0x0575**: a per-key "key currently down" bitmap table (one byte per key, set to 1 when the key's counter hits the stride). Same shape (0x15 bytes/row) as the 0x04F7 counter table. **INFERRED** from being written exactly when the counter reaches stride and never read inside this function.
- **0x584a–0x584c: `mov r7, 0x1D ; lcall fcn.00001b46`** — calls the frame processor with **r7 = IRAM[0x1D] (the row index 0–5)**. **VERIFIED**
  - r5 at this point: the prologue saved r5 into 0x1E and did not reload it before this call. Tracing r5: at 0x57db r5→[0x1E]; nothing in 0x57eb–0x584a writes r5. So **r5 = the original r5 (column-byte index 0–15)**. **VERIFIED** — confirmed by 0x584f `mov r5, 0x1E` immediately after (restoring r5 from [0x1E] for the next call), which only makes sense if r5 was already [0x1E].
- 0x584f–0x5853: `mov r5, 0x1E ; mov r7, 0x1D ; lcall fcn.0000a6de` — calls the effect-engine dispatcher with **r7 = row, r5 = col** (same pair). **VERIFIED**
  - `fcn.0000a6de` (0xa6de, 591 bytes): stores r7→[0x0EE7], r5→[0x0EE8]; gates on `[0x0BA6]==0`; then dispatches on `[0x0F83]`:
    - 7 → `ljmp 0xB20B`
    - 9 → `ljmp 0x9F7E`
    - 0x0C → `ljmp 0x8690`
    - 0x13 → `lcall fcn.00009260`
    - 4 → `ljmp 0xB986`
    - default → ret. **VERIFIED**
  - So **the r7:r5 passed to fcn.0000a6de (i.e. row:col) becomes [0x0EE7]:[0x0EE8]** — the same pair fcn.00001b46 wrote at entry. **VERIFIED**

### Arm B (bit 0x2b.4 clear, 0x5897–0x596a) — key-up / idle processing
- 0x5897–0x58b5: builds pointer **0x0575 + row*0x15 + col** (`add a,#0x75 ; addc a,#0x05`), reads `[0x0575 + row*0x15 + col]`; if zero → `ljmp 0x594E`; else falls through. **VERIFIED**
- 0x58ba–0x58d6: recompute the 0x0575 pointer, `movx a,@dptr ; inc a ; movx @dptr,a` → increments it (a release counter?). **VERIFIED**
- 0x58d7–0x58f2: recompute, `cjne a,#0x06, 0x591F` — if it equals 6, take the "fully released" branch at 0x58f5; else 0x591f. **VERIFIED**
- 0x58f5–0x591e (counter==6): recompute the **0x04F7** pointer (counter table), `clr a ; movx @dptr,a` → clears the key's counter at [0x04F7 + row*0x15 + col]; then `mov r5,0x1E ; mov r7,0x1D ; lcall fcn.00002AAA` (the second-tier dispatcher) followed by `lcall fcn.0000B9F9`. **VERIFIED**
- 0x591f–0x596a: the "not yet 6" branch — recomputes 0x0575 pointer, compares against 0xC8 (200); if ≥ 200 clears it (resets the release counter); else increments again. Then `ret`. **VERIFIED**

### Summary of fcn.000057db
- It is the **per-key debounce + event dispatcher**, called once per (row, col) bit of the key matrix.
- Two tables in XDATA:
  - **0x04F7** — per-key **press counter** (0x15 bytes/row × N rows). Incremented on each scan while key held; cleared on release.
  - **0x0575** — per-key **down-state latch** (same shape). Set to 1 when press counter hits `[0x0C4F]` (stride); used as a release counter on key-up (increments to 6 then clears the press counter).
- When a key crosses the press threshold it calls **`fcn.00001b46`** with (r7=row, r5=col) → which stores row→[0x0EE7], col→[0x0EE8].
- Then it calls **`fcn.0000a6de`** with the same (r7=row, r5=col) → which also stores them to [0x0EE7]:[0x0EE8] and dispatches the active effect.
- **`[0x0C4F]` (stride)** = the press-count threshold for actuation. **INFERRED** (semantics) but **VERIFIED** as the value compared against the per-key counter.
- **The r7:r5 reaching `fcn.0000a6de` (and thus `[0x0EE7]:[0x0EE8]`) are the row and column indices of the physical key (0–5 and 0–15), NOT frame bytes.** **VERIFIED**

---

## 3. Callers of fcn.00001b46

`grep` for `lcall fcn.00001b46` / `121b46` finds exactly **one** caller:
- **0x584c inside fcn.000057db**, with r7=IRAM[0x1D] (row 0–5), r5=IRAM[0x1E] (col 0–15). **VERIFIED**

There are no other call sites. The continuation region 0x1e6e–0x218a is reached only by internal `jmp @a+dptr` from 0x1e99 (itself reached by `ljmp` from the `0x1e6e` block which is dead code relative to the 0x1e6d ret — `0x1e6e` is the byte *after* the ret; radare lists it with `DATA XREF from fcn.00002aaa @ 0x2d96`). **VERIFIED** — so fcn.00001b46's 0x1e6e+ tail is actually entered from `fcn.00002aaa`, not from fcn.00001b46 proper. This means the 0x1e6e–0x218a block is a **shared helper** (effect-index update routine) invoked by the matrix-release path (`fcn.00002aaa`, called at 0x5915 in fcn.000057db's release arm). **VERIFIED** for entry; the labelling as "part of fcn.00001b46" is a radare artifact.

---

## 4. Report-0x09 frame byte offsets → state registers

Frame base = XDATA 0x1100. **frame[N] = [0x1100 + N]**.

| State register | Frame byte offset(s) that can reach it | Path | Confidence |
|---|---|---|---|
| **XDATA 0x0F3F (effect index)** | **none from the live frame** | The only writer reachable from fcn.00001b46's tail is 0x2114: `mov a,#0x12` (constant). Other writers (0x21f? in `fcn.0000B1B7`-family, 0xd178, 0xd3c2, 0xbbee, 0x9f7e-path via `fcn.0000B161` with r7 from `[0x1131]`) all either write constants or derive from the **staged config block / flash read-back**, not the live 0x1100 frame. The staged block at 0x11C1 is itself built by `fcn.0000B161`/`fcn.00006efd` from **flash** (via the copy engine), not from the live frame. | **VERIFIED** (no live-frame path) |
| **XDATA 0x0F54 (mode)** | **none from the live frame** | All writers (0x0fa3 region in `fcn.00000F45`, 0x100e, 0x4409, 0x4502, 0x7e86, 0xd266, 0xd1e0, 0xd3ba, 0xdb60) write constants (0x35/0x45/0x55/0x01/0x02) or copy from `[0x0B9F]` (the saved-mode mirror). `fcn.00000F45` is reachable from fcn.00001b46 (0x1e52 `ljmp fcn.00000F45`) but only with r7=row,r5=col — and `fcn.00000F45` reads `[0x0F54]` to *compare*, then overwrites it with a constant. No path copies a live frame byte into 0x0F54. | **VERIFIED** (no live-frame path) |
| **XDATA 0x0F64 (brightness)** | **none from the live frame** | Writers: 0x041e (constant 0x3C default), 0x10b6 (reads `[0x0F64]` itself — decrement), 0x359e (decrement), 0x828c (clr a → 0), 0x82d2. None read from the 0x1100 frame buffer. | **VERIFIED** (no live-frame path) |
| **XDATA 0x1130 command block (20 bytes)** | **frame[0x30..0x43]** (bytes 0x30–0x43, 20 bytes) | `fcn.000096ce` (0x96ce) copies up to 20 bytes from the copy-engine source pointer `[0x0F75]:[0x0F76]:[0x0F77]` (space:hi:lo) into 0x1130, then sets `[0x0F7F]=1`. The source pointer is set up by `fcn.00007253` (the cmd-0x0b/0x0c/0x0d handler at 0x87dc/0x8810) with r6:r7 = 0x11:0x00..0x08 — i.e. **the source space is XDATA 0x11xx**, meaning the source is the frame buffer itself (0x1100+offset). For cmd 0x0a, `fcn.00005001` (0x87c6) is the fast path and does not use 0x1130. **However**, the boot-block path at 0xff00 (entered when `[0x1150]==0x09` and magic matches) programs flash from the frame directly. So 0x1130 is filled from the frame only via the cmd-0x0b/0x0c/0x0d apply path, where the source offset within the frame is controlled by `[0x0F75..0x0F77]` — set by the caller, typically pointing at frame[0x30] (the 20-byte command sub-block). | **INFERRED** (the 0x30 base is conventional, not hard-coded in 0x96ce; the source pointer is what determines it) |
| **XDATA 0x11C1 staged config block (19 bytes)** | **none from the live frame directly** | `fcn.0000B161` (0xb161) writes a fixed header (0x5A, 0xD1, 0x05) to 0x11C1–0x11C3, then `mov a,r5 ; movx @dptr,a` puts r5 (the caller's "layer id") at 0x11C4, then `lcall fcn.0000D210`. The 19-byte body is filled by the copy engine from flash, not from the live frame. Callers (`fcn.00009D2F` at 0x9d2f, 0x9d56) pass r7=`[0x1131]` (a byte from the **0x1130 command block**, which itself came from the frame via the 0x96ce path) and r5 = 0xAA or 0x55 (constants). So 0x11C1's only live-frame-derived byte is the **layer id at 0x11C4, sourced from frame[0x31]** (via [0x1131]). | **INFERRED** (0x11C4 ← frame[0x31], single byte) |
| **XDATA 0x0EE7:0x0EE8 (frame pointer pair)** | **none — set to physical row:col** | fcn.00001b46 writes r7→[0x0EE7], r5→[0x0EE8] at entry; the only caller (fcn.000057db @ 0x584c) passes row:col. Not frame-derived. | **VERIFIED** |
| **XDATA 0x0EE9 (slot index)** | **none — computed** | `[0x0EE7] + [0x0EE8]*6` (0x1b73). Derived from row:col, not frame. | **VERIFIED** |
| **XDATA 0x0F50 (lighting param/channel)** | **none from the live frame** | Set by `fcn.00007E86` (constants from `movc` table at 0xA47E), `fcn.00009DED`, and the `fcn.00000F00` family. None read from 0x1100. | **VERIFIED** (no live-frame path) |

### Reachable frame→state paths that DO exist
| Frame byte | Reaches | Via | Confidence |
|---|---|---|---|
| **frame[0x50]** ([0x1150]) | command dispatch | `fcn.00008765` reads it directly | **VERIFIED** |
| **frame[1]** ([0x1101]) | magic check (must == 0x05) | 0x87af | **VERIFIED** |
| **frame[2]** ([0x1102]) | magic check (must == 0x75) | 0x87b8 | **VERIFIED** |
| **frame[0x31]** ([0x1131]) | → [0x11C4] (staged layer id), via [0x1131]→r7→`fcn.0000B161`→r5 path at 0x9d2f/0x9d56 | **INFERRED** (one indirect hop) |
| **frame[0x30..0x43]** | → 0x1130 command block (via copy engine when cmd 0x0b/0x0c/0x0d fires and source pointer = 0x1130) | **INFERRED** (source pointer is set by caller) |
| **entire 519-byte frame** | → flash (via boot-block 0xff00 when cmd==0x09 and magic 5A/A5 matches) | **VERIFIED** (the boot-block program path) |

### Registers with NO live-frame path (explicit)
- **0x0F3F (effect index)** — no live-frame path. **VERIFIED**
- **0x0F54 (mode)** — no live-frame path. **VERIFIED**
- **0x0F64 (brightness)** — no live-frame path. **VERIFIED**
- **0x0EE7:0x0EE8** — set to physical key row:col, not frame. **VERIFIED**
- **0x0F50 (lighting param)** — no live-frame path. **VERIFIED**

These registers are changed only by: (a) constants in the firmware, (b) the staged-config apply path (which reads back from *flash* via the copy engine, not the live frame), or (c) the boot-block flash-program path (which writes flash, after which a reboot/reload updates them). There is **no code path that copies a byte from the live 0x1100 frame buffer directly into 0x0F3F, 0x0F54, 0x0F64, or 0x0F50**.

---

## 5. Confidence roll-up

| Claim | Confidence |
|---|---|
| fcn.00001b46 entry stores r7→[0x0EE7], r5→[0x0EE8] | VERIFIED |
| fcn.00001b46 short-circuits on bit 0x27.5 (sets [0x0F3E]\|=2) | VERIFIED |
| fcn.00001b46 computes [0x0EE9] = [0x0EE7] + [0x0EE8]*6 | VERIFIED |
| fcn.00001b46 reads 4-byte records from table at 0x0BD4 (+[0x0F50]+[0xE0]+[0x0EE9]*4) | VERIFIED |
| fcn.00001b46 writes per-slot status to 0x0B17 + [0x0EE7]*0x15 + [0x0EE8] | VERIFIED |
| fcn.00001b46 tail (0x1e6e–0x218a) is entered from fcn.00002aaa, not from 0x1e6d | VERIFIED |
| 0x0F3F is written with constant 0x12 at 0x2114 (no frame data) | VERIFIED |
| No `mov PSW` in fcn.00001b46 (bank 0 throughout) | VERIFIED |
| fcn.000057db is the per-key matrix walker (row 0–5, col 0–15) | VERIFIED |
| fcn.000057db calls fcn.00001b46 at 0x584c with r7=row, r5=col | VERIFIED |
| fcn.000057db calls fcn.0000a6de at 0x5853 with r7=row, r5=col | VERIFIED |
| Table at 0x0575 = per-key "down" latch (set to 1 when counter == [0x0C4F]) | INFERRED |
| Table at 0x04F7 = per-key press counter | INFERRED (semantics) / VERIFIED (address) |
| [0x0C4F] = actuation stride/threshold | INFERRED |
| r7:r5 passed to fcn.0000a6de become [0x0EE7]:[0x0EE8] | VERIFIED |
| Frame buffer = XDATA 0x1100–0x1303 (519 bytes) | VERIFIED |
| [0x1150] = frame[0x50] = command byte | VERIFIED |
| Magic frame[1]==0x05, frame[2]==0x75 gates the cmd-0x09 path | VERIFIED |
| frame[0x31] → [0x1131] → [0x11C4] (staged layer id) | INFERRED (one hop) |
| frame[0x30..0x43] → 0x1130 command block (via copy engine, cmd 0x0b/0x0c/0x0d) | INFERRED (source-pointer dependent) |
| No live-frame path to 0x0F3F, 0x0F54, 0x0F64, 0x0F50 | VERIFIED |
| Boot-block 0xff00 programs flash from the live frame (cmd 0x09 + magic) | VERIFIED |
| fcn.00001b46 has exactly one caller (fcn.000057db @ 0x584c) | VERIFIED |

---

## 6. Key address reference

| XDATA | Role | First cited |
|---|---|---|
| 0x04F7 | per-key press counter table (0x15/row) | 0x57f0 |
| 0x0575 | per-key down-latch table (0x15/row) | 0x5833 |
| 0x0B17 | per-(slot,channel) status byte table | 0x1c71 |
| 0x0BA8 | "config changed" latch | 0x1d51 |
| 0x0BA9 | mode selector (==2 special) | 0x1c86 |
| 0x0BAC | current layer id mirror | 0x1d5c |
| 0x0BC1 | repeat-counter ceiling | 0x1bff |
| 0x0BC2 | active-command flag | 0x1b78 |
| 0x0BD2 + slot | per-slot repeat counter | 0x1bf3 |
| 0x0BD4 + ([0x0F50]+[0xE0]+slot*4) | 4-byte per-slot recipe record | 0x1b9c |
| 0x0C45 | live-config busy gate (set to 1 on entry) | 0x1b59 |
| 0x0C47 | sub-dispatch selector (1/2/3/4+) | 0x1ca3 |
| 0x0C48 | macro slot / mode arg | 0x1cea |
| 0x0C49 | media key code | 0x1d35 |
| 0x0C4A | layer id (new) | 0x1d2e |
| 0x0C4D | per-call gate (0 or 0x69 → skip) | 0x1cdb |
| 0x0C4E | debounce/hold counter (threshold 0xC8) | 0x1ec6 |
| 0x0C4F | actuation stride/threshold | 0x5824 |
| 0x0EE7:0x0EE8 | frame pointer pair (row:col) | 0x1b46 |
| 0x0EE9 | slot index = [0x0EE7]+[0x0EE8]*6 | 0x1b74 |
| 0x0EEA | frame status latch | 0x1b7d |
| 0x0EEF | 'S' register selector / saved r7 | 0x1c9b |
| 0x0F1D..0x0F20 | 4-byte staging buffer | 0x2124 |
| 0x0F3D | lighting flags (bit0 = apply, bit5 = staged) | 0x211b |
| 0x0F3E | staged dirty flag (bit1) | 0x1b51 |
| 0x0F3F | effect index | 0x210f |
| 0x0F50 | lighting param / channel | 0x1b90 |
| 0x0F54 | mode | 0x203d |
| 0x0F64 | brightness | 0x828c |
| 0x0F83 | effect/mode selector (dispatches fcn.0000a6de) | 0xa6ef |
| 0x1100 | frame buffer base (519 bytes) | 0x87ae |
| 0x1130..0x1143 | 20-byte command block | 0xb68b |
| 0x1150 | command byte (frame[0x50]) | 0x8765 |
| 0x11C1..0x11D3 | 19-byte staged config block | 0xb16f |

---
## 7. CORRECTION (2026-09-24, parent agent): frame buffer base is 0x1150, not 0x1100

The report above uses base 0x1100 and puts the command byte at frame[0x50].
That is WRONG (off by 0x50). Empirical proof: the working color-write frames
carry the command at payload[0], and the dispatcher fcn.00008765 reads the
command from [0x1150] (e.g. 0x87bf: `dptr=0x1150; a=[0x1150]; cjne #0x0a;
lcall fcn.00005001`). payload[0] lands at [0x1150], so:

  frame[N] == XDATA[0x1150 + N]

Corrected offsets:
- command byte = frame[0] ([0x1150])            VERIFIED (works live)
- the ISP magic check reads frame[4]==0x05, frame[5]==0x75 ([0x1154]/[0x1155])
  (0x87ae-0x87ba: dptr advances 0x1150->0x1154 before the xrl #0x05 check)
- 0x1130 and 0x11C1 are BELOW the frame buffer (0x1150): they are workspace
  blocks, NOT aliases of frame bytes. Any claim mapping frame offsets onto
  0x1130/0x11C1 via "base 0x1100" must be re-derived; treat as UNKNOWN.

The structural findings (row:col contract for fcn.00001b46/fcn.0000a6de,
no live-frame path to 0x0F3F/0x0F54/0x0F64, matrix tables 0x04F7/0x0575,
recipe table 0x0BD4, per-key debounce flow) are unaffected by this offset
correction and stand as VERIFIED.
