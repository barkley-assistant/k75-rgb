# Test Batch 2 — Retired

This test plan was prepared before the command-path audit. **Do not execute its
prebuilt images or the old `setkey` / `sidelight` probes as a test batch.**
Its assumptions that report-0x09 command `0x0A` carries positional per-key
RGB and command `0x08` carries six side LEDs were not supported by the firmware
or the live observations. Both named probes are disabled. The one-shot `'S'`
register sweep did not reach its apply threshold and was inconclusive.

The generated `analysis/t2-*.bin` images remain in the repository as historical
research artifacts, **not validated test vectors**. No new pattern, brightness,
or side-light behavior should be inferred from their filenames.

## Current test gate

1. Trace the USB feature report through the correct staging buffer and prove
   the complete packet prefix, length, command route, and apply condition.
2. For per-key testing, trace the 21×6×RGB table at XDATA `0x0379` to physical
   LED output and establish a non-destructive complete baseline.
3. For the case light, identify its output and writer independently; the
   white/ice-blue observation has not been isolated to a single packet.
4. Only then build one evidence-backed candidate and run it with the operator
   observing a known-good baseline. Do not save unverified state.

Report-0x09 command `0x04` remains **forbidden**: it previously produced a
lights-off state that known USB commands failed to recover; Fn+Esc was required.
Fn+PgUp's ordinary keys-off state is distinct and did allow a later red write.

See [protocol-audit.md](../protocol/protocol-audit.md) for disassembly offsets and live-test
evidence. The original plan remains available in git history.
