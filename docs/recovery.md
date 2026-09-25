# K75 RGB — Recovery & Safety

Read this before probing the device.

## Recovery hatches (both proven)

| Situation | Fix |
|---|---|
| Lighting wedged by a bad config write | **Fn+Esc, hold ~3s** — factory reset of the lighting engine. Verified twice (rainbow returns). |
| Fn keys stop changing effects after a wedge | Same: Fn+Esc. Fn+Backspace / Fn+↑ / replug do **not** recover. |
| Worst case / bricked app | Full ISP reflash via sinowisp from the verified backups (below). |

## Firmware backups (MD5-verified across 4 dumps)

- `fw/k75_full.bin` — full stock firmware
  MD5 `be1f5410a2f97d4143b96ed56fbb1c11`
- `fw/k75_boot.bin` — bootloader
  MD5 `3e0ebd0c440af5236d7ff8872343f85d`

## Hard rules (from the user, still binding)

1. **No invented packets.** Every packet sent must trace to a live capture
   or a disassembly offset. A plausible packet that does nothing is a failed
   deliverable.
2. **Never send flash erase (0x45) on the ISP channel.** (0x45 on the
   report-0x09 lighting channel is a legitimate mode command — context
   dependent, but never on ISP.)
3. **Keep the stock-firmware backup safe** before any ISP operation.

## Lessons learned (all three cost a lights-off incident or worse)

1. **cmd 0x04 is not a read.** It reloads data-flash into the live lighting
   state. Fired while flash held a stale bad config, it blanked all lights.
   Fn+Esc recovered, but: **never send report-0x09 cmd 0x04**.
2. **Malformed config + save (0x06) persists garbage.** The wedge then
   survives replugs and Fn keys — only Fn+Esc fixes it. Only save a config
   image after it has been visually verified.
3. **Fn+Esc restores the live state but does not rewrite data-flash.**
   The stale bad config can come back on the next reload. If a bad config
   was ever saved, prefer writing a known-good image + save to overwrite it
   (once the good image is verified).

## Safe probing checklist

- Config-write tests: one byte at a time, no save, Fn+Esc ready.
- The 72-byte factory image (`analysis/config-default-72.bin`) is the
  known-good baseline — the control test is part of every session.
- Report-0x09 command `0x08` has a traced host-to-RAM-matrix path and two
  visibly verified slots: slot 0 controls Esc, slot 63 controls the UK `;`
  key. The complete red baseline and each isolated green slot lit the keys
  for about two seconds; bounded repeated reports kept them lit until
  streaming stopped. The case stayed rainbow and is not mapped by this test.
  The old 18-byte `sidelight` probe remains disabled.