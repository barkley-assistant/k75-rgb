# K75 RGB — Firmware Architecture

Instruction-level decode of the stock firmware (`fw/k75_full.bin`,
MD5 `be1f5410a2f97d4143b96ed56fbb1c11`). Addresses are file offsets (8051
CODE memory, non-banked). Primary source: `analysis/disasm_v2.txt`.

## 1. The effect command processor (fw 0x66–0x2xx)

Radare2's linear sweep never disassembled this — the UART vector at 0x63
(`ljmp 0xa748`) killed the sweep at 0xB9F8. Forced re-disassembly of the
vector region revealed the full **0x0F54 mode dispatch** plus
`fcn.0000a274` command-block calls with effect ops
(0x0A/0x1B/0x08/0x02/0x13/0x15/0x17…).

## 2. State registers (live XDATA)

| Register | Meaning | Host path |
|---|---|---|
| 0x0F3F | effect index (0–15) | **none** — only firmware constants or config load |
| 0x0F83 | effect code (dispatch value for fcn.0000a6de) | none |
| 0x0F54 | mode byte | **none** (0x01/0x25/0x35/0x45/0x55) |
| 0x0F64 | brightness (default 0x3C = 60) | **none** |
| 0x0F22 | mode flags | derived from 0x0F54 |
| 0x0F3E | apply gate (0x10 set = apply staged) | via 'S' staging only |
| 0x0F5B | data-flash engine arm (0x5A → fire) | indirect |
| 0x0F80 | staged flash op | indirect |
| 0x0EFF | register-write target select | 'S' protocol only |
| 0x0EE7:0x0EE8 | physical key row:col (matrix scan output) | none |
| 0x0EEE–0x0EF0 | flash-op frame pointer (r6:r7:r5) | from caller |

**The decisive finding:** 0x0F3F, 0x0F54 and 0x0F64 have **no live-frame
path**. Their only writers are firmware constants (Fn-key handlers) and the
config-load path. Host control of mode/brightness/speed therefore goes
through the **config region write** (cmd 0x0a) — nothing else reaches them.

### Mode map (0x0F54 → 0x0F22, from the 0x66 processor)

| 0x0F54 | flags | observed live |
|---|---|---|
| 0x01 | OR 0x01 | |
| 0x25 | 0x00 | |
| 0x35 | 0x01 | static rainbow (factory default) |
| 0x45 | 0x02 | **multi-color wave** (confirmed 2026-09-24) |
| 0x55 | 0x03 | |

## 3. Effect index → code → handler chain

1. Effect index at 0x0F3F, settable via the 0x1130 register block
   `[0x5A, 0xAC, <index>]` consumed by `fcn.0000bbcd`
   (cmd 0xAC → byte → 0x0F3F; cmd 0xAA → byte → 0x1155).
2. Code lookup table **0xDB76**:
   `01 02 03 04 05 07 08 09 0a 0b 0c 0d 0f 10 11 13` (16 codes).
3. Mode→effect table **0xA40A**.
4. `fcn.0000a6de` dispatches on **[0x0F83]** (recognized: 0x07/0x05/0x08/
   0x0B/0x13) and stores r7:r5 → [0x0EE7:0x0EE8] (the row:col pair for
   per-key effects).
5. Per-effect param records at CODE 0xC000/0xC100 (and profile variants
   0xC200/0xC300): 4-byte records holding timing/param values
   (0x29/0x35/0x2B/0x39/… — mode-adjacent constants).

## 4. Data-flash engine

- Op staged at **0x0F80**; arm when **[0x0F5B] == 0x5A** → fires
  `fcn.0000d8e8`.
- Ops: **0x52 read, 0x54 write buffer, 0x56 commit**.
- Data-flash address space: 0x00–0x32.
- Validator `fcn.0000b35e`; byte-level primitive `fcn.000084e2`.
- **Per-key grid**: 21×6 = 126 bytes transferred per op; live table at
  XDATA 0x05F4 + col×36 (pointer pair [0x08F1:0x08F2]).
- Save handler `fcn.00007393`: serializes the CODE 0xAD7A template table →
  XDATA 0x0DB2. Sparse mask — 0xFF markers at indices 0, 9, 14, 15, 21.

## 5. Per-key matrix

- Walker `fcn.000057db` (0x57db–0x596a): iterates row 0–5 × col 0–15,
  calls `fcn.00001b46` (key state machine: debounce, 0x0BD4 recipe table,
  0x0B17 status table) at 0x584c and `fcn.0000a6de` at 0x5853 with
  row:col in r7:r5.
- 6 rows × 16 cols = 96 slots; populated slots = 81 keys (matches the
  physical key count; no 0x51 bound check exists — searched exhaustively).
- The [0x0EE7]:[0x0EE8] pair is **row:col from the matrix scan**, not a
  host-controlled frame pointer (this kills an earlier wrong theory from the
  other agent's M5).

## 6. Command-message block (0x11C1)

19 bytes at 0x11C1–0x11D3:
`[0x5A magic, type, sub, op, flags(2), payload..., checksum]`
Checksum at 0x11D8 = `0xFF − sum(bytes[0x11C2..0x11C1+len])`, length r5 = 0x13.
Register engine `fcn.0000b7d2` builds it from REG → 0x0EFF + r7 = value;
called only at 0x7540 (inside `fcn.0000b684`, the report-0x09 engine) with
r7 = 0xBB.

## 7. Direct-LED (cmd 0x08)

`fcn.00007108`: 6 iterations × 3 bytes from `frame[r5 + i×3]` → XDATA
0x0379 + i×3 (18 bytes total). Consumer: effect engine at 0x2dd6 reads the
0x0379 zone with stride 0x12; status flag at [0x0375].
The RK M75/Kreo family header (`09 08 00 00 01 00 <len> <RGB...>`) is
misaligned for this — that's why the family-format direct-LED test failed.

## 8. Flash read command (cmd 0x04) internals

`fcn.00008402`: stages opcode 0x52, iterates frame[8..135] through the flash
op; `frame[9] == 1` → writes 0x13 response marker at frame[10].
**The transfer is flash → live table — an internal config reload, not a
host-visible read.** GET 0x09 returns only the 8-byte ACK echo (verified
live). This is why firing it re-applies whatever config is in flash.