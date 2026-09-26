# K75 RGB — Documentation Index

Reverse-engineering the proprietary RGB protocol of the RedThunder K75
(8051-based Sino Wealth keyboard) and building Linux tooling.

## Where to start

| Doc | What's in it |
|---|---|
| [`../ROADMAP.md`](../ROADMAP.md) | Feature-completeness tracker: status + progress per feature. |
| [`reference/feature-map.md`](reference/feature-map.md) | Detailed feature table with paths, commands, and verification status. |
| [`reference/recovery.md`](reference/recovery.md) | **Safety net:** Fn+Esc factory reset, backups, hard rules. Read before touching the device. |

## Protocol & USB

| Doc | What's in it |
|---|---|
| [`protocol/protocol-audit.md`](protocol/protocol-audit.md) | **Current correction and test gate** — what the firmware and live tests actually establish. Read before running any probe. |
| [`protocol/protocol.md`](protocol/protocol.md) | Working USB protocol: reports, command table, verified sequences. |
| [`protocol/software-protocol.md`](protocol/software-protocol.md) | Vendor-tool protocol decode (official software behaviour). |
| [`protocol/config-region.md`](protocol/config-region.md) | The 72-byte config profile: confirmed offsets, hypotheses, live results. |

## Firmware internals

| Doc | What's in it |
|---|---|
| [`firmware/architecture.md`](firmware/architecture.md) | Effect engine, registers, data-flash, per-key matrix — instruction-level decode with addresses. |
| [`firmware/rendering-architecture.md`](firmware/rendering-architecture.md) | Transient-vs-persistent rendering, the `0x0F59` countdown, `0x0F83=0x13` persistence path, register-block ingress. |
| [`firmware/case-frame-stepping.md`](firmware/case-frame-stepping.md) | Case light: frame-synced animation stepping (1 frame = 1 step). |
| [`firmware/effect-table.md`](firmware/effect-table.md) | Decoded effect-code → renderer map, Fn+Tab cycle order, palettes, predictions. |

## Mapping & reference

| Doc | What's in it |
|---|---|
| [`reference/key-map.md`](reference/key-map.md) | Vendor LED index (`col×6+row`) vs the 21×6 firmware matrix; verified slot pins. |

## Device identity

| Field | Value |
|---|---|
| Product | RedThunder K75 (short name "K75"; firmware rev marker `Fw=26`) |
| Manufacturer | SINO WEALTH (Sino Wealth Electronics, Shanghai) |
| USB wired | VID `0x258A` PID `0x019D` — "Gaming Keyboard" / "Gaming KB" |
| USB wireless (2.4 GHz dongle) | VID `0x3554` PID `0x0150` |
| USB descriptor strings | "SINO WEALTH", "Gaming Keyboard", bcdDevice `0001`, "BY Tech" |
| MCU | 8051-compatible, 64 KB flash + 4 KB ISP boot region; exact part number not stated in firmware or vendor tool (likely Sino Wealth SH68F family — unconfirmed) |
| Layout | en-GB / UK ISO (16×6 key grid, 81 keys + gaps) |

Source: USB string descriptors in `../fw/` dumps and the vendor tool's
`Dev/kb/KB.ini`.

## Plans & test sessions

| Doc | What's in it |
|---|---|
| [`plans/PLAN.md`](plans/PLAN.md) | Remaining work, concrete steps with acceptance criteria. |
| [`plans/test-plan-4.md`](plans/test-plan-4.md) | Next live-test batch (lit-key effect map, persistence re-test, save re-verify, case determinism). |
| [`plans/test-plan-3.md`](plans/test-plan-3.md) | Completed batch (key map, case walk, cadence experiment) — results recorded. |
| [`plans/test-plan-2.md`](plans/test-plan-2.md) | Retired earlier batch; kept for history. |

## Raw artifacts

- `../analysis/disasm_v2.txt` — full 8051 disassembly of the stock firmware
  (3.1 MB, the primary RE source)
- `../analysis/key-led-map.txt` — vendor LED ID table
- `../analysis/config-dumps/*.bin` — 72-byte config test images
- `../analysis/video-evidence-2026-09-26/` — case-strip video frames,
  viewer, and frame-sample analysis
- `../fw/` — firmware dumps (full + bootloader, MD5-verified)
- `../tools/hidra_probe/` — Rust library + CLI (`k75`) and read-only probes
- `../tools/hidra_probe/legacy-probes/` — historical probes, unmaintained;
  may encode disproven assumptions

## The 30-second version

1. **Colour write + flash save were observed live** (`0x0a`/`0x0b`/`0x06`)
   and persisted across replug **and 2.4 GHz wireless mode**.
2. **Per-key matrix display verified** (`0x08`, 126 slots); 14 key slots
   visually pinned, remaining keys mapped by vendor-table reference.
3. **Case light is host-driveable**: Fn+Tab walks 11 case stops, and each
   `0x08` matrix frame steps the case animation once.
4. **Persistence via effect `0x13`** is firmware-traced; one live re-test
   remains (see `plans/test-plan-3.md`).
5. Read [`protocol/protocol-audit.md`](protocol/protocol-audit.md) before
   running any probe; the old `setkey`/`sidelight` programs are retired.