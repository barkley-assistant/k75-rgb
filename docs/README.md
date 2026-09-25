# K75 RGB — Documentation Index

Reverse-engineering the proprietary RGB protocol of the RedThunder K75
(8051-based Sino Wealth keyboard) and building a Linux driver + CLI + GUI.

## Read this first

| Doc | What's in it |
|---|---|
| [protocol-audit.md](protocol-audit.md) | **Current correction and test gate:** what the firmware and live tests actually establish; read before running probes. |
| [protocol.md](protocol.md) | Working USB notes; older assumptions are corrected in the audit. |
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

1. **Color + save were observed live.** Full-payload red changed the keys;
   save persisted a color setting across a power-cycle and wireless mode.
   The red keys still showed a moving wave, not a static effect.
2. **The old mode claim was retracted.** `+0x0E` feeds `0x0CC8`
   (speed-related), while `+0x1F` feeds mode register `0x0F54`.
3. **Per-key control was visually verified.** A complete command-`0x08` red
   frame lit the keys; slot 0 turned Esc green and slot 63 turned the UK `;`
   key green against otherwise red keys. The key lights went off after about
   two seconds, while a bounded repeated-frame test kept them lit until
   streaming stopped. The case side light stayed rainbow; see
   [protocol-audit.md](protocol-audit.md).
4. **Read [protocol-audit.md](protocol-audit.md)** before running a probe;
   the old `setkey` and `sidelight` programs are disabled.