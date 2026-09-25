# K75 RGB — Reverse-Engineering Status

Read the [protocol audit](docs/protocol-audit.md) before running an old probe.

## Confirmed live

- Full-color payload via report-0x09 `0x0A` + `0x0B` changed the key colors.
  After Fn+PgUp turned the key lights off, a red payload produced red keys
  with a dim moving left-to-right red wave. This did **not** establish static
  mode or independent per-key addressing.
- `0x06` save persisted a color setting across power-cycle and wireless mode.
  Save only after visually verifying the staged state.
- Fn+Esc held for several seconds recovered the keyboard from earlier
  lights-off incidents. Report-0x09 `0x04` caused a different, severe
  lights-off state that known USB commands did not restore; **do not send it**.

## Confirmed in firmware, not yet host-verified

- An internal `0x08` handler at `0x7108` writes 21×6 RGB entries to XDATA
  `0x0379..0x04F2`. Its USB frame route, physical key indices, and any relation
  to the side light remain unknown.
- The internal `0x0A` handler at `0x9308` performs a two-page flash-buffer
  transfer, not a positional per-key write. The host-to-internal staging path
  needs further tracing before building a new packet.
- Profile byte `+0x0E` feeds speed-related register `0x0CC8`; byte `+0x1F`
  feeds mode register `0x0F54`. The earlier “0x35 static, 0x45 wave” claim
  was invalidated by the factory default already being a wave.

## Open

- Host route to the 21×6 RGB table and its physical key mapping.
- Side/case light output path and an isolated command for it. A temporary
  white/ice-blue observation was not attributable to a single packet.
- Mode, speed, effect, and brightness control through a verified host path.
  Short report-0x06 `'S'` writes staged data but did not apply it, so the
  register sweep ruled nothing out.

The old `setkey` and `sidelight` probes are disabled. The old test batch was
retired in [docs/test-plan-2.md](docs/test-plan-2.md). The stock firmware
backups and recovery rules are in [docs/recovery.md](docs/recovery.md).
