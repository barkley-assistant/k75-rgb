# legacy-probes/ — historical experiments

These binaries are **retired** and no longer compile as part of the crate.
They are kept for reference: each one was a step in the reverse-engineering
process, and several encode assumptions that were later disproven or
superseded.

**Do not run these against the keyboard without re-checking their packet
construction** against `../docs/protocol/protocol-audit.md`. Known offenders:

- `setkey` / `sidelight` — built on disproven sparse-payload assumptions
- `setcolor` / `setcolor2` — superseded by the verified `0x0a`/`0x0b` path
  (`save_color`) and the `0x08` matrix path (`k75`)
- `raw-sender` (former `src/main.rs`) — sends arbitrary hex bytes; no
  safety gating, use only for deliberate experiments

The current supported surface is:

- `src/bin/k75.rs` — main CLI (matrix frames, key map, effect blocks)
- `src/bin/get09_readonly.rs` — read-only feature-response probe
- `src/bin/readcfg.rs` — config read, fail-closed on the forbidden `0x04`
- `src/bin/getled.rs` — LED state read
- `src/bin/save_color.rs` — verified colour write + flash save path
  (`0x0a`/`0x0b`/`0x06`), pending port into the library