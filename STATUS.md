# K75 RGB — Reverse-Engineering Status (honest, 2026-09-24)

All findings now consolidated in [`docs/`](docs/) — start with the
[index](docs/README.md), the [protocol reference](docs/protocol.md), and the
[plan](docs/PLAN.md).

## Verified (live-tested, traceable)

1. **Color change + save** — report 0x09: cmd `0x0a` (write) + `0x0b` (apply)
   + `0x06` (save). Persists across power-cycle and 2.4G wireless.
   Tools: `setcolor2`, `save_color`.
2. **Mode byte** — config `+0x0E` confirmed live 2026-09-24: 0x35 = static
   rainbow, 0x45 = wave. (`cfgwrite` + `config-mode45-72.bin`.)
3. **Complete report-0x09 command table** — 0x03/0x04/0x05 flash ops, 0x06
   save, 0x08 direct-LED (6×RGB → 0x0379), 0x0a config write, 0x0b/0x0c/0x0d
   apply. Full decode in [docs/protocol.md](docs/protocol.md).
4. **Effect architecture decoded end-to-end** — 0x0F3F index → 0xDB76 code
   table → 0x0F83 → handler chain; **no live-frame path** to mode/brightness/
   effect registers (host route = config write). See
   [docs/architecture.md](docs/architecture.md).
5. **Recovery proven** — Fn+Esc (hold ~3s) factory reset; sinowisp ISP
   reflash from MD5-verified backups.

## In progress

- **Speed + brightness offsets** — candidates built, next live session is
  scripted ([docs/PLAN.md](docs/PLAN.md) Session 2).
- **Side/case underglow** — cmd 0x08 direct-LED path decoded, untested live
  (Session 3).

## Known limits

- **No host-visible config read** — cmd 0x04 is an internal reload (probed;
  caused a lights-off incident when flash held a stale bad config). Driver
  state must be tracked host-side.

## Lessons (full details in [docs/recovery.md](docs/recovery.md))

- cmd 0x04 is not a read — never fire it unattended.
- Malformed config + save persists garbage; only Fn+Esc recovers.
- Fn+Esc restores live state but does **not** rewrite data-flash.