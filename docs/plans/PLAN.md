# K75 RGB — Remaining Work

The [protocol audit](../protocol/protocol-audit.md) and the
[roadmap](../../ROADMAP.md) are
the source of truth. The earlier live-test plan is [retired](test-plan-2.md).

1. **Map the remaining keys.** The report-0x09 ingress and command `0x08`
   matrix byte offsets are firmware-traced; a complete red frame visibly lit
   the keys. Slot 63 isolated the UK `;` key and slot 0 isolated Esc, agreeing
   with the vendor LED IDs at those two positions. See [key-map.md](../reference/key-map.md):
   the vendor LED index is `col×6 + row` over a 16×6 physical grid, while the
   firmware matrix is 21×6 (126 slots), so 30 slots are non-key (dead or the
   case-light chain). Host slot = vendor LED index is the leading hypothesis,
   confirmed only at 0 and 63. Complete it with one green slot per observation
   against a red baseline.
2. **Characterize transient rendering.** The `0x0F59=0xFA` countdown is now
   fully explained — see [rendering-architecture.md](../firmware/rendering-architecture.md).
   Expiry force-loads effect `0x01` (`0x35C6`); selecting effect `0x13` arms
   the persistence flag `0x0C4D=0x69` (`0x49B9`). The live persistence recipe
   (stage matrix, then Fn+Tab to the last effect) is the headline test of
   [test-plan-3.md](test-plan-3.md). Bounded streaming remains the proven
   fallback.
3. **Trace the case output separately.** It stayed rainbow through all of
   the `0x08` key-matrix tests. Firmware trace shows it is a separate animation
   engine on `0x0F99..0x0F9F` with no live-USB writer; it is config-driven via
   `0x0BBF` (profile `+0x0F`). See [protocol-audit.md](../protocol/protocol-audit.md).
4. **Characterize mode, speed, effect, and brightness** only after their
   staging/apply paths are established. The old `+0x0E` static/wave label and
   short `'S'` register sweep did not validate those controls.
5. **Build the Linux Rust CLI and API** around demonstrated commands; expose
   unknown controls only after visual verification. The proposed Electron GUI
   must not advertise unimplemented lighting behavior.

Do not send report-0x09 `0x04`, ISP erase `0x45`, or flash firmware. Save only
after confirming a visible result. Keep stock backups and Fn+Esc recovery
instructions available. Commit and push reproducible findings as they land.
