# Effect selection from the host — register-block channel (traced, 2026-09-26)

Traced through the firmware (`analysis/disasm_v2.txt`). Everything below is
disassembly-offset-traceable; the **carrier report is still not fully
pinned**, so `k75 effect` remains send-gated.

## The 20-byte register block

Buffer: XDATA `0x1130..0x1143` (20 bytes).

Grammar (parser `fcn.0000bbcd`, called from the dispatcher `fcn.0000b684`
when `block[0] == 0x5A`):

```
offset 0:  0x5A                    (tag)
offset 1:  command byte
offset 2:  value byte
offset 3+: padding (17 bytes)
```

Commands:

| cmd | Effect | Trace |
|---|---|---|
| `0xAC` | `0x1155 = value` (secondary register; then `setb 0x2A.4`) | `0xbbd3: add a,#0x54; jz 0xbbe7` |
| `0xAA` | `0x0F3F = value` (the mode/state register itself) | `0xbbd7: add a,#0x02; jnz 0xbbf5` |
| other | `0x0F3D |= 0x20` (fallback flag) | `0xbbf5` |

So `0x5A 0xAC <idx> + pad` writes `0x1155 = <idx>` — the traced effect-index
write — and `0x5A 0xAA <v>` writes the mode register `0x0F3F`.

## Block assembly (byte-stream assembler)

The block is NOT one report. It is assembled byte-by-byte from the raw USB
receive stream:

```
assembler (0x9722..0x9766, fed from fcn.000096ce via the USB ISR at 0xa778):
  if (0x0F3F == 0x22) AND (0x0F54 == 0x01):      ; armed precondition
      0x1130[0x0F74] = 0x0EF6                    ; current received byte
      0x0F74++
      if 0x0F74 >= 0x14 (20): 0x0F7F = 1         ; block complete
  else:
      0x0F7F = 0; 0x0F74 = 0                     ; disarmed: reset

dispatcher (fcn.0000b684):
  if 0x0F7F == 1:
      if 0x1130[0] == 0x5A: fcn.0000bbcd(0x1130)   ; parse
      else: fcn.0000d2f6(...)                      ; other block handler
      0x0F7F = 0
```

Every received byte lands in `0x0EF6` (`fcn.000096ce`: `0x0EF6 = IRAM 0xAA`,
the USB FIFO byte), so **any report whose payload streams through the USB
interrupt feeds the assembler while it is armed**.

## The arming chicken-and-egg

The assembler only accepts bytes while `0x0F3F == 0x22 && 0x0F54 == 0x01`.
But `0x5A 0xAA 0x22` (which would set `0x0F3F = 0x22`) needs the assembler
to be *already* armed. Therefore the initial arming comes from a different
route — internal firmware writers of `0x0F3F` (`0x25a7`, `0x45a2`, `0xb4ca`,
`0xbbf1`, `0x2114`, `0xa368`) or the Fn+Tab keyboard path or config load.
**This is the missing link** that keeps the register-block channel from
being a clean standalone host write. Candidates to pin next:

1. What sets `0x0F3F = 0x22` on the Fn+Tab path (trace the key handler).
2. The config-region mirror (`0x0BC1`/`0x0BC5` were written from the
   per-effect table loader `0x6257/0x6260` — config load may arm it).

## The control-transfer channel (parallel, "AH"/"AZ")

SETUP dispatch at `0x14A9` (bRequest in `0x114A`):

- bRequest `0x01` → writes `"AH"` (0x41 0x48) to `0x0FA3:0x0FA4`, returns
  a block (length from wLength) — the host **GET** channel.
- bRequest `0x02` → writes `"AZ"` (0x41 0x5A) — the second GET channel.

These are what `get09_readonly` reads. The official tool carries a table
of wValue constants near `0x24b29b`: `0x17FF, 0x1600, 0x1000, 0x0F00,
0x1700` — the vendor-request family over this channel. `0x17FF` walks the
effect table (`0x906d` transfer loop). The exact wValue → effect-index
encoding over this channel is the remaining unpinned bit of the
effect-selection story.

## The two-block command protocol (traced, 2026-09-26)

The firmware runs **two** `0x5A`-tagged block channels:

### Phase 1 — arm (block B, buffer `0x11E0`)

- Fed by the same USB byte pump; when the pump's position counter
  `0x0F78:0x0F79` reaches `IRAM[0x07]` (= 182), flag `0x2A.2` fires
  (`0x971e`).
- Dispatcher `0xd15f`: reads `0x11E0`, and if `block[0] == 0x5A` calls
  `fcn.0000249b` (else `fcn.0000b490`).
- `fcn.0000249b` is the command-value dispatcher: it reads block fields
  `0x11E2..0x11E5` and branches on the value:
  - value `0x22` → **`0x0F3F = 0x22`** (`0x25e9`) — the arming write
  - value `0x12` → `0x0F52:0x0F53 = 0`, `0x0F3D |= 0x01`
  - value `0x11` → `0x0F52:0x0F53 = 0`, `0x0F3D |= 0x10`
  - (gated on `0x0F54 ∈ {0x25, 0x35, 0x45, 0x55}`)
- `0x0F54 = 0x01` (the second arming condition) is written by
  `fcn.00004409` (reached via the `0x40xx` ajmp table).

### Phase 2 — execute (block A, buffer `0x1130`)

As documented above: while armed (`0x0F3F==0x22 && 0x0F54==0x01`), each
pump byte appends to `0x1130`; at 20 bytes `0x0F7F` fires; `0xb684` →
`fcn.0000bbcd` parses `0x5A <cmd> <value>` (`0xAC` → `0x1155`, `0xAA` →
`0x0F3F`).

### What remains unpinned

1. The USB-level framing that places `0x5A` at block-B offset `0xE0`
   (224) and routes pump bytes past position 182 — i.e. the exact report
   ID and payload layout for phase 1. Both blocks are filled by indirect
   (register-pointer) copies, which the static disassembly cannot resolve;
   the fill loop itself is `fcn.0000b63f` / `fcn.000096ce` (byte pump).
2. Whether phase-2 bytes can ride in the same 520-byte report as phase 1.

Net: the full two-phase command architecture is mapped, but the packet
layout still needs either a live capture of the official tool or one more
round of indirect-addressing analysis. Until then, `k75 effect` stays
send-gated.

## Status

| Item | State |
|---|---|
| `0x5A 0xAC <idx>` block grammar | ✅ pinned (`0xbbcd`) |
| Block assembly mechanism (20-byte, precondition-gated) | ✅ pinned (`0x9722`/`0xb684`) |
| Arming route for `0x0F3F=0x22` | 🔬 traced: block B (`0x11E0`, value `0x22` → `fcn.0000249b`); USB framing still unpinned |
| Carrier report for the block bytes | ❌ unpinned (any streamed byte while armed) |
| "AH"/"AZ" control-transfer channel | ✅ pinned (`0x14A9`) |
| wValue family used by official tool | ✅ found in tool (`0x17FF/0x1600/0x1000/0x0F00/0x1700`) |
| wValue → effect-index encoding | ❌ unpinned |

Net: the block format is confirmed, but a host-usable effect-selection
write still needs the arming route. Until then `k75 effect` stays
send-gated (dry-run print only), per the no-unverified-packet rule.