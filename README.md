# k75-rgb

Reverse-engineering the **RedThunder K75** keyboard's proprietary RGB protocol and
building Linux tooling to control it — with the goal of a small Rust daemon + HTTP
API + Electron GUI as the final product.

## Documentation

All findings live in [`docs/`](docs/) — start with the
[documentation index](docs/README.md).

| Doc | Covers |
|---|---|
| [docs/protocol.md](docs/protocol.md) | Verified USB protocol: reports, command table, working sequences |
| [docs/architecture.md](docs/architecture.md) | Firmware internals: effect engine, registers, data-flash, matrix |
| [docs/config-region.md](docs/config-region.md) | The 72-byte config profile: confirmed offsets + live test results |
| [docs/recovery.md](docs/recovery.md) | Fn+Esc factory reset, ISP reflash, backups, hard rules |
| [docs/PLAN.md](docs/PLAN.md) | Remaining work broken down session by session |

## Status

- **M1 — color change** ✅ verified live (wired + 2.4G wireless)
- **Persistence (save)** ✅ verified — survives power-cycle and wireless mode
- **Mode control** ✅ `+0x0E` config byte confirmed (0x35 = rainbow, 0x45 = wave)
- **Speed / brightness** 🔬 candidates built, live mapping in progress
- **Side/case underglow** ⏳ cmd 0x08 direct-LED path decoded, untested live

## Running

```sh
cd tools/hidra_probe
cargo build --release
sudo ./target/release/save_color FF 00 00   # set all keys red + save
sudo ./target/release/cfgwrite ../../analysis/config-mode45-72.bin  # write a config image
```

Requires `sudo` (raw HID access to the vendor interface).

## Layout

- `docs/` — canonical documentation (protocol, architecture, config, plan)
- `tools/hidra_probe/` — Rust CLI probes (uses [hidra](https://crates.io/crates/hidra))
- `analysis/` — raw RE artifacts: disassembly, decode reports, test images, session logs
- `fw/` — stock firmware dumps (MD5-verified, for ISP recovery)

## Recovery

**Fn+Esc (hold ~3s)** = factory reset, fixes any wedged lighting config.
Full ISP reflash via [sinowisp](https://crates.io/crates/sinowisp) from the
backups in `fw/`. **Never send `0x45` (flash erase) on the ISP channel.**
Details in [docs/recovery.md](docs/recovery.md).

## License

[GPL-3.0](LICENSE) — free software, so others can build on the RE work.

Note: the `fw/` directory contains proprietary firmware dumps (RedThunder / Sino
Wealth) included strictly for reverse-engineering research; those binary blobs are
**not** covered by this project's GPL license.