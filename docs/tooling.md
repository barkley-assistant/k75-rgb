# Tooling — `k75` CLI and `k75d` daemon

Both binaries wrap `tools/hidra_probe/src/lib.rs` (`#![forbid(unsafe_code)]`),
the single source of truth for frame formats. Nothing else in the tree talks
USB directly.

## Device access

The vendor lighting interface (`258A:019D`, interface 1) is a hidraw node
owned by root. Everything that touches the device — `--send`, and the daemon
itself — therefore runs as root **or** behind a udev rule. There is no
workaround; that is the Linux permission model.

```sh
# optional udev rule (then no sudo needed):
# /etc/udev/rules.d/99-k75.rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="258a", ATTRS{idProduct}=="019d", MODE="0660", GROUP="plugdev"
```

## `k75` — CLI

```text
k75 matrix baseline [--send [--repeat 1..20] [--interval 100..10000]]
k75 matrix slot <0..125> [--send ...]
k75 save <RRGGBB | R G B> [--send]
k75 save --pattern <file> [--send]
k75 effect <0..0x13> [--trace]
k75 map
```

- `matrix` frames are the **verified** path (report `0x09`, command `0x08`).
  One frame = one case-animation step; the CLI spaces resends ≥100 ms.
- `save` runs the **verified** write (`0x0a`) → apply (`0x0b`) → flash save
  (`0x06`) sequence. `--pattern` loads one `RRGGBB` per line, slot order,
  `#` comments allowed; the slot geometry (slot `i` at payload `2+i*3`) is
  traced from `fcn.00007108` but not yet visually re-confirmed.
- `effect` is **dry-run only**: the register-block grammar
  (`0x5A 0xAC <idx>`) is firmware-pinned, but the USB carrier frame that
  arms and delivers it is not (see
  [protocol/effect-selection.md](protocol/effect-selection.md)).
- `map` prints the slot→key table (verified at 14 points, hypothesis
  elsewhere).

## `k75d` — daemon

```text
k75d [--socket /tmp/k75d.sock] [--allow-writes]
```

JSON-lines over a unix socket; one response per request. The socket is
chmod'd `0666` so a root daemon serves non-root clients (GUI, scripts).

| op | params | notes |
|----|--------|-------|
| `ping` | — | device presence + write state |
| `device` | — | VID/PID/interface, or why it's absent |
| `map` | — | slot→key table |
| `matrix_baseline` | `send`, `repeat`, `interval_ms` | verified frame; writes gated |
| `matrix_slot` | `slot`, `send`, … | verified frame; writes gated |
| `save` | `send`, `color` **or** `pattern` | verified sequence; writes gated |
| `effect` | — | always refused (carrier unpinned) |

### Safety model

- Read-only unless started with `--allow-writes`.
- `effect` is refused unconditionally — Fn+Tab does effects natively.
- The forbidden report-`0x09` command `0x04` (lights-off hazard seen in live
  tests) does not exist in this codebase, and report-`0x05` ISP command
  `0x45` (flash erase) is likewise absent. No flashing, ever.