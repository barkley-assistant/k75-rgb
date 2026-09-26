# k75-rgb

Reverse-engineering the **RedThunder K75** keyboard's proprietary RGB protocol and
building Linux tooling to control it — with the goal of a small Rust daemon + HTTP
API + Electron GUI as the final product.

## Status

Current feature-completeness tracking lives in [`ROADMAP.md`](ROADMAP.md)
(per-feature status + progress) with the detailed paths and notes in
[`docs/feature-map.md`](docs/feature-map.md).

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
- **Mode/speed mapping** 🔬 `+0x0E` was misidentified as the mode byte; later disassembly maps it to speed-related register `0x0CC8`. `+0x1F` feeds mode register `0x0F54`. No static-mode control has been established.
- **Per-key / side light** 🔬 report-0x09 command `0x08` is traced from USB receive to a 21×6 RGB matrix and visually verified: slot 0 lights Esc green and slot 63 lights the UK `;` key green against otherwise red keys. Each frame expires after about two seconds; bounded host streaming kept it lit until the sends stopped. The case remained rainbow. Other key slots and independent case-light control remain unmapped; see [protocol audit](docs/protocol-audit.md).

## Running

```sh
cd tools/hidra_probe
cargo build --release --bin matrix08 --bin get09_readonly

# matrix frames (verified path)
./target/release/matrix08 matrix baseline                 # offline preview
./target/release/matrix08 matrix slot 63                  # offline preview
./target/release/matrix08 map                             # slot -> key hypothesis table
sudo ./target/release/matrix08 matrix baseline --send     # send (transient)
sudo ./target/release/matrix08 matrix slot 63 --send --repeat 16

# traced-only effect-index register block (does NOT send; see below)
./target/release/matrix08 effect 0x13
```

The next live test batch (key-map completion, Fn+Tab effect map, and the
persistence recipe) is scripted in [docs/test-plan-3.md](docs/test-plan-3.md).

The protocol core now lives in `src/lib.rs` — a `#![forbid(unsafe_code)]`
library encoding the verified matrix-frame format and the traced effect-index
register block, each gated behind explicit markers. The `matrix08` binary is a
thin CLI over it:

- `matrix baseline|slot <N>` builds a frame; `--send` writes it, `--repeat
  1..20` resends at 500 ms (keeps keys lit, still transient ~2 s).
- `effect <index>` prints the traced `0x5A 0xAC <index>` register block but
  **refuses to send**: its carrier report is not yet pinned to a verified
  command, so sending would risk a "plausible packet that does nothing".

`matrix08 baseline --send` is a volatile test that writes the complete red
matrix and does **not** save it. `matrix08 slot 63 --send` changed only the
UK `;` key to green in a live test; `--repeat 16` bounds a half-second host
refresh experiment that kept the keys lit while it ran. The key lights turn
off again when refresh stops; the case light stayed rainbow. Do not run the
older save/config probes as part of this experiment.

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