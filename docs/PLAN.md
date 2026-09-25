# K75 RGB — Remaining Work

The [protocol audit](protocol-audit.md) and [current status](../STATUS.md) are
the source of truth. The earlier live-test plan is [retired](test-plan-2.md).

1. **Map the remaining keys.** The report-0x09 ingress and command `0x08`
   matrix byte offsets are firmware-traced; a complete red frame visibly lit
   the keys. Slot 63 isolated the UK `;` key and slot 0 isolated Esc, agreeing
   with the vendor LED IDs at those two positions. Use the same full baseline
   and one changed slot per visual observation; do not extrapolate every UK
   ISO position from two checks.
2. **Characterize transient rendering.** The `0x0F59=0xFA` countdown and a
   bounded host refresh were observed to keep the keys lit during streaming,
   then let them turn off after it stopped. Determine whether a safely
   reachable static/persistent mode exists before relying on continual refresh.
3. **Trace the case output separately.** It stayed rainbow through all of
   the `0x08` key-matrix tests. Find its actual driver, state, and host-reachable
   writer; an earlier white/ice-blue observation does not identify a command.
4. **Characterize mode, speed, effect, and brightness** only after their
   staging/apply paths are established. The old `+0x0E` static/wave label and
   short `'S'` register sweep did not validate those controls.
5. **Build the Linux Rust CLI and API** around demonstrated commands; expose
   unknown controls only after visual verification. The proposed Electron GUI
   must not advertise unimplemented lighting behavior.

Do not send report-0x09 `0x04`, ISP erase `0x45`, or flash firmware. Save only
after confirming a visible result. Keep stock backups and Fn+Esc recovery
instructions available. Commit and push reproducible findings as they land.
