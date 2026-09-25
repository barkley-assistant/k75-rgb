# K75 RGB — Remaining Work

The [protocol audit](protocol-audit.md) and [current status](../STATUS.md) are
the source of truth. The earlier live-test plan is [retired](test-plan-2.md).

1. **Trace the host route.** Follow the report-0x09 USB receive path through
   XDATA `0x1150` and determine if/how it can reach the separate `0x08FA`
   dispatcher. Prove any complete report prefix, length, and apply gate from
   captured traffic or instruction offsets before constructing a new packet.
2. **Trace the key output.** Follow the 21×6 RGB table at `0x0379` through
   rendering and LED output. Map internal slots to physical UK ISO keys with
   a controlled, known-safe full-frame baseline before trying an isolated key.
3. **Trace the case output separately.** Find its actual driver, state, and
   host-reachable writer. The white/ice-blue observation does not identify a
   command. Do not repurpose the `0x08` matrix handler without proof.
4. **Characterize mode, speed, effect, and brightness** only after their
   staging/apply paths are established. The old `+0x0E` static/wave label and
   short `'S'` register sweep did not validate those controls.
5. **Build the Linux Rust CLI and API** around demonstrated commands; expose
   unknown controls only after visual verification. The proposed Electron GUI
   must not advertise unimplemented lighting behavior.

Do not send report-0x09 `0x04`, ISP erase `0x45`, or flash firmware. Save only
after confirming a visible result. Keep stock backups and Fn+Esc recovery
instructions available. Commit and push reproducible findings as they land.
