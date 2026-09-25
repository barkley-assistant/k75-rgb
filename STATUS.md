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

## Firmware-traced ingress and visually verified per-key control

- Report-0x09 feature reception stages eight-byte USB chunks at XDATA
  `0x1100` into `0x08FA` (`0x8906`, `0x8765`, `0x7253`, `0xB0A8`).
  `0x1150` is transfer state, **not** a host payload byte.
- Host report offset 1 (`0x08FB`) selects command `0x08`, which writes
  21×6 RGB entries from report offsets 8..385 to XDATA `0x0379..0x04F2`.
  The renderer reads a completion marker and loads countdown `0x0F59=0xFA`.
- The complete red command-`0x08` frame visibly made the keys red. Changing
  only **slot 63 to green** turned the UK-layout `;` key next to L green while
  all other keys stayed red; this was observed twice. Changing **slot 0** to
  green lit Esc. Both match their vendor LED IDs, but the remaining IDs are
  not yet validated as host slots. The case side stayed rainbow. The key
  lights went off after about two seconds. During the Esc test the rainbow
  looked static, but the operator could not attribute that to the packet.
- Sixteen identical RAM-matrix frames sent 500 ms apart kept the colours lit
  during streaming; the keys went off again after it stopped. Streaming is
  verified as a temporary sustain method, **not** a proven persistent mode.
- The separate `0x0A` handler at `0x9308` performs a two-page flash-buffer
  transfer, not a positional per-key write. Its exact visible-state semantics
  remain open; do not use a sparse `0x0A` payload as a key map.
- Profile byte `+0x0E` feeds speed-related register `0x0CC8`; byte `+0x1F`
  feeds mode register `0x0F54`. The earlier “0x35 static, 0x45 wave” claim
  was invalidated by the factory default already being a wave.

## Open

- Map the remaining internal RGB slots to the physical UK ISO keys, using
  complete baseline reports with only one differing slot per observation.
- Find a safely reachable sustained/static mode, or use a deliberate bounded
  host refresh policy; sending one frame does not persist the effect.
- Side/case light output path and an isolated command for it. A temporary
  white/ice-blue observation was not attributable to a single packet; the
  matrix tests left the case rainbow. **Firmware trace now shows the case
  light is a separate animation generator on `0x0F99..0x0F9F` with no
  live-USB-frame writer — it is config/profile-driven via `0x0BBF`
  (profile `+0x0F`).** See the case-light section in
  [protocol-audit.md](docs/protocol-audit.md). Untested live.
- Mode, speed, effect, and brightness control through a verified host path.
  Short report-0x06 `'S'` writes staged data but did not apply it, so the
  register sweep ruled nothing out.

The old `setkey` and `sidelight` probes are disabled. The old test batch was
retired in [docs/test-plan-2.md](docs/test-plan-2.md). The stock firmware
backups and recovery rules are in [docs/recovery.md](docs/recovery.md).
