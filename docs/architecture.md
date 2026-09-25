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

| Register | Firmware role | Host control status (not a complete writer audit) |
|---|---|---|
| 0x0F3F | effect index (0–15) | no independent live host write demonstrated |
| 0x0F83 | effect code (dispatch value for fcn.0000a6de) | unproven |
| 0x0F54 | mode byte | no independent live host write demonstrated (0x01/0x25/0x35/0x45/0x55) |
| 0x0F64 | brightness (default 0x3C = 60) | no independent live host write demonstrated |
| 0x0F22 | mode flags | derived from 0x0F54 |
| 0x0F3E | apply gate (0x10 set = apply staged) | `'S'` staging path, full apply still unverified |
| 0x0F5B | data-flash engine arm (0x5A → fire) | indirect |
| 0x0F80 | staged flash op | indirect |
| 0x0EFF | register-write target select | `'S'` staging path |
| 0x0EE7:0x0EE8 | physical key row:col (matrix scan output) | unproven |
| 0x0EEE–0x0EF0 | flash-op frame pointer (r6:r7:r5) | from caller |

The examined writers of `0x0F3F`, `0x0F54`, and `0x0F64` include firmware
constants (Fn-key handlers) and config-load code. No independent host-to-live
register frame has been demonstrated. This does **not** rule out other
host-reachable writers; the `'S'` staging path still needs full tracing.

### Mode map (0x0F54 → 0x0F22, from the 0x66 processor)

| 0x0F54 | flags | observed live |
|---|---|---|
| 0x01 | OR 0x01 | factory default: moving rainbow wave |
| 0x25 | 0x00 | effect varies with other registers |
| 0x35 | 0x01 | effect varies with other registers |
| 0x45 | 0x02 | effect varies with other registers |
| 0x55 | 0x03 | effect varies with other registers |

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

## 7. Internal RGB matrix writer (command 0x08 in the `0x08FA` dispatcher)

`0x7B35` calls `0x7108` with base XDATA `0x08FA` and source offset 8. The
outer loop runs 21 times (`0x723E–0x724C`), the inner loop 6 times
(`0x711D–0x7127`), with three channels per slot. Source address:
`0x08FA + 8 + row×18 + column×3 + channel`; destination address:
`0x0379 + row×18 + column×3 + channel` (126 slots, 378 bytes).
The effect engine accesses `0x0379` with the same 18-byte stride at `0x2DD6`.
**Neither physical-key order nor side-light membership is established.** The
18-byte `sidelight` probe misidentified this table and is disabled. Also, this
internal dispatcher is not yet connected to the external report-0x09 host
frame: `0x87BF–0x87F2` handles the `0x1150` host buffer separately. See
[protocol-audit.md](protocol-audit.md).

## 8. Flash read command (cmd 0x04) internals

`fcn.00008402`: stages opcode 0x52, iterates frame[8..135] through the flash
op; `frame[9] == 1` → writes 0x13 response marker at frame[10].
**The transfer is flash → live table — an internal config reload, not a
host-visible read.** GET 0x09 returns only the 8-byte ACK echo (verified
live). This is why firing it re-applies whatever config is in flash.