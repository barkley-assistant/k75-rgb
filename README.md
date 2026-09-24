# k75-rgb

Reverse-engineering the **RedThunder K75** keyboard's proprietary RGB protocol and
building Linux tooling to control it — with the goal of a small Rust daemon + HTTP
API + Electron GUI as the final product.

## Status

**POC COMPLETE** (2026-09-24): color *change* and *save* (persistence) both verified
live, in wired **and** 2.4G wireless mode.

| Milestone | State |
|-----------|-------|
| M1 — one confirmed write that changes LED color | ✅ done |
| M2 — full command set (effects/brightness/speed) | in progress |
| Change + save persistence POC | ✅ done |
| Case/side underglow light | ⏳ in progress |

## The working protocol (verified)

All control is via **report 0x09** (HID feature report) on the vendor interface
(`258a:019d`, `/dev/hidraw1`), 519-byte payload frames.

```
1. cmd 0x0a  → per-key color write   (frame: [0x0a, R,G,B, R,G,B, ...])
2. cmd 0x0b  → apply to PWM          (frame: [0x0b, 0,0,0, ...])
3. cmd 0x06  → save to flash         (frame: [0x06, 0,0,0, ...])
```

Full details in [`PROTOCOL-WORKING.md`](PROTOCOL-WORKING.md) and
[`PROTOCOL-DECODED.md`](PROTOCOL-DECODED.md).

## Layout

- `tools/hidra_probe/` — Rust CLI probes (uses [hidra](https://crates.io/crates/hidra))
  - `setcolor2` — set color (0x0a + 0x0b)
  - `save_color` — set + save (0x0a + 0x0b + 0x06)
  - `rv`, `ring`, `cmd`, `sweep`, `statediff`, `setreg` — protocol probes
- `analysis/disasm_v2.txt` — ANSI-stripped 8051 disassembly (primary RE source)
- `fw/` — stock firmware dumps (see recovery section)
- `HANDOVER.md` — full reverse-engineering handover notes

## Running

```sh
cd tools/hidra_probe
cargo build --release
sudo ./target/release/save_color FF 00 00   # set all keys red + save
```

Requires `sudo` (raw HID access to the vendor interface).

## Recovery (proven)

Full firmware is backed up and re-flashable via [sinowisp](https://crates.io/crates/sinowisp)
(Sinowealth SH68F90 ISP):

- `fw/k75_full.bin` — 64 KiB, MD5 `be1f5410a2f97d4143b96ed56fbb1c11`
- `fw/k75_boot.bin` — 4 KiB, MD5 `3e0ebd0c440af5236d7ff8872343f85d`

ISP sequence: `0x75` enter ISP → `0x55` enable firmware → `0x52` read / `0x57` write.
**Never send `0x45` (flash erase) on the ISP channel.**

## License

Reverse-engineering work. No license chosen yet — proprietary firmware dumps are
included for research purposes only.