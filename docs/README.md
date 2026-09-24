# K75 RGB — Documentation Index

Reverse-engineering the proprietary RGB protocol of the RedThunder K75
(8051-based Sino Wealth keyboard) and building a Linux driver + CLI + GUI.

## Read this first

| Doc | What's in it |
|---|---|
| [protocol.md](protocol.md) | The **verified** USB protocol: reports, frames, command table, working sequences. Everything live-tested against the real board. |
| [architecture.md](architecture.md) | Firmware internals: effect engine, registers, data-flash, per-key matrix. Instruction-level decode with addresses. |
| [config-region.md](config-region.md) | The 72-byte config profile layout — what's confirmed, what's hypothesis, live test results. |
| [recovery.md](recovery.md) | Safety net: Fn+Esc factory reset, ISP reflash, backups, hard rules. **Read before touching the device.** |
| [PLAN.md](PLAN.md) | What's left, broken down into concrete steps with acceptance criteria. |

## Raw artifacts

- `../analysis/disasm_v2.txt` — full 8051 disassembly of the stock firmware
  (3.1 MB, the primary RE source; verified sound against `../fw/k75_full.bin`)
- `../analysis/fn1b46-decode.md` — subagent decode of the per-key matrix
  walker (`fcn.00001b46` + `fcn.000057db`), incl. frame-base correction
- `../analysis/test-session-1.md` — log of the first live config test session
- `../analysis/*.bin` — pre-built config test images for live sessions
- `../fw/` — firmware dumps (full + bootloader, MD5-verified)
- `../tools/` — Rust probe binaries (hidra)

## The 30-second version

1. **Color + save already work.** `setcolor2` changes colors, `save_color`
   persists them across power-cycles and wireless mode.
2. **Mode control works.** Config byte `+0x0E` confirmed live (0x35 = static
   rainbow, 0x45 = wave).
3. **Still being mapped:** speed + brightness offsets in the config region,
   and the side/case underglow zone (candidate: cmd 0x08 direct-LED path).
4. **Recovery is proven:** Fn+Esc (hold ~3s) factory-resets lighting; full
   ISP reflash from the firmware backup is the nuclear option.