# k75-rgb

Reverse-engineering the **RedThunder K75** keyboard's proprietary RGB protocol and
building Linux tooling to control it — with the goal of a small Rust daemon + HTTP
API + Electron GUI as the final product.

## Status

- **Colour write** ✅ verified live (wired + 2.4 GHz wireless)
- **Flash save** ✅ verified — survives power-cycle and wireless mode
- **Per-key matrix display** ✅ verified — 126 slots, 15 keys visually pinned
- **Case light** ✅ host-driveable — Fn+Tab walks 11 stops, each `0x08` frame steps the case animation
- **Persistent custom matrix** 🔬 traced end-to-end, one live re-test left
- **Host effect selection** 🔬 carrier packet unpinned

Full tracker: [`ROADMAP.md`](ROADMAP.md) · detailed paths: [`docs/reference/feature-map.md`](docs/reference/feature-map.md).

## Documentation

All findings live in [`docs/`](docs/) — start with the [documentation index](docs/README.md).

| Doc | Covers |
|---|---|
| [protocol audit](docs/protocol/protocol-audit.md) | What firmware + live tests actually establish (read before probing) |
| [protocol](docs/protocol/protocol.md) | Verified USB protocol: reports, command table, sequences |
| [firmware architecture](docs/firmware/architecture.md) | Effect engine, registers, data-flash, matrix |
| [rendering architecture](docs/firmware/rendering-architecture.md) | Transient vs persistent rendering, `0x0F83=0x13` path |
| [case frame stepping](docs/firmware/case-frame-stepping.md) | Case light: 1 frame = 1 animation step |
| [key map](docs/reference/key-map.md) | 126 slots ↔ physical keys, verified pins |
| [config region](docs/protocol/config-region.md) | 72-byte config profile layout |
| [recovery](docs/reference/recovery.md) | Fn+Esc reset, ISP reflash, hard rules |
| [plan](docs/plans/PLAN.md) | Remaining work with acceptance criteria |

## Running

```sh
cd tools/hidra_probe
cargo build --release --bin k75 --bin get09_readonly

# matrix frames (verified path)
./target/release/k75 matrix baseline              # offline preview
./target/release/k75 matrix slot 63               # offline preview
./target/release/k75 map                          # slot -> key hypothesis table
sudo ./target/release/k75 matrix baseline --send  # send (transient ~2 s)
sudo ./target/release/k75 matrix slot 63 --send --repeat 16 --interval 500

# traced-only effect-index register block (does NOT send; carrier unpinned)
./target/release/k75 effect 0x13
```

The protocol core lives in `src/lib.rs` — a `#![forbid(unsafe_code)]` library
encoding the verified matrix-frame format, the vendor key map, and the traced
effect-index register block, each gated behind explicit markers. The `k75`
binary is a thin CLI over it:

- `matrix baseline|slot <N>` builds a frame; `--send` writes it; `--repeat
  1..20` resends; `--interval 100..10000` sets the frame spacing in ms
  (default 500).
- `effect <index>` prints the traced `0x5A 0xAC <index>` register block but
  **refuses to send**: its carrier report is not yet pinned to a verified
  command, so sending would risk a "plausible packet that does nothing".

The next live test batch (key map, Fn+Tab effect map, persistence recipe) is
scripted in [test-plan-3](docs/plans/test-plan-3.md). Requires `sudo` (raw
HID access to the vendor interface).

## Layout

- `docs/` — canonical documentation, grouped: `protocol/`, `firmware/`,
  `reference/`, `plans/`
- `tools/hidra_probe/` — Rust library + `k75` CLI (uses [hidra](https://crates.io/crates/hidra));
  `legacy-probes/` holds historical experiments kept for reference
- `analysis/` — raw RE artifacts: disassembly, config dumps, video evidence,
  session logs
- `fw/` — stock firmware dumps (MD5-verified, for ISP recovery)

## Recovery

**Fn+Esc (hold ~3s)** = factory reset, fixes any wedged lighting config.
Full ISP reflash via [sinowisp](https://crates.io/crates/sinowisp) from the
backups in `fw/`. **Never send `0x45` (flash erase) on the ISP channel, and
never use report-0x09 command `0x04`.** Details in
[recovery](docs/reference/recovery.md).

## License

[GPL-3.0](LICENSE) — free software, so others can build on the RE work.

Note: the `fw/` directory contains proprietary firmware dumps (RedThunder / Sino
Wealth) included strictly for reverse-engineering research; those binary blobs are
**not** covered by this project's GPL license.