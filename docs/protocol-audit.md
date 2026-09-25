# Protocol audit: host writes versus the LED matrix

This is a correction to earlier probe interpretations. It is based on `fw/k75_full.bin` (8051 CODE offsets below), the Rust probe source, and the operator's visual observations. No device writes were made during this audit.

## Verified firmware paths

- The report-0x09 command processor at `0x7A83–0x7AC0` dispatches a command stored at XDATA `0x08FB`. Its `0x08` branch reaches `0x7B35`, which calls `0x7108` with a pointer to XDATA `0x08FA` and data offset `8`. This identifies an **internal dispatch path**, not by itself the USB feature-report layout or proof that a bare report-0x09 `0x08` reaches that path.
- At `0x711D–0x7127`, the inner loop runs six times; at `0x723E–0x724C`, the outer loop terminates after 21 iterations. At `0x712B–0x7156` and the matching green/blue paths, source addressing uses `base + 8 + outer * 18 + inner * 3 + channel`. Destinations at `0x715A–0x7167`, `0x71B6–0x71C3`, and `0x7212–0x721F` are `0x0379 + outer * 18 + inner * 3 + channel`. The complete table is 126 RGB slots / 378 bytes, XDATA `0x0379..0x04F2`. The routine finishes by writing `0x5A` to `0x0E34` at `0x724C–0x7252`.
- `0x2DD6–0x2DF4` accesses `0x0379` with an 18-byte outer stride and a 3-byte inner stride; `0x86D3–0x86F6` also addresses that table. This supports a key/effect-matrix role; it does **not** identify the physical LED ordering or the side/case-light output.
- `0x7B2A–0x7B33` invokes `0x9308` for the `0x0A` internal command. That function at `0x934F–0x939A` drives a two-page, 256-byte-per-page write through the `0x54` flash-buffer operation; it is **not** a positional RGB-triple-to-key writer. Whether the externally sent `0x0A` frame takes this exact internal branch on every transfer must still be traced through USB staging.
- The *other* command processor at `0x87BF–0x87F2` branches on the XDATA `0x1150` host buffer: `0x0A` calls the `'S'` staging parser at `0x5001`; `0x0B` enters `0x7253`. It has no explicit `0x08` branch in that range. The relationship between XDATA `0x1150` and `0x08FA` must be established before encoding a host-side `0x08` packet.

- A renderer branch at `0x2E8A–0x2ED7` reads `0x0379 + row×18 + column×3` and writes `0x08FA + row×6 + column`. This is another internal use of the `0x08FA` region; it is not safe to treat that region as a verbatim USB frame. The conditions under which this branch runs remain to be traced.

- At `0xA65B–0xA66B`, a USB request gated on bytes `0x1149 = 0x09` and `0x114A = 0x03` calls `0x6AEE`. That routine reads `0x08FA..0x0901` and copies those bytes into `0x1108..0x110F` (`0x6AFA–0x6B86`). This is **internal-to-USB-response direction**, not evidence of the converse USB-input-to-matrix route. The rest of the request/response and any separate receive route still require tracing.

The offline checker `python3 analysis/verify_matrix_path.py` validates the
stock firmware's instruction signatures and internal matrix address ranges.
It never opens a HID device; it cannot establish host packet layout or physical
LED mapping.

## What the live observations actually prove

- A full red `0x0A` payload followed by `0x0B` visibly made keys red after the operator used Fn+PgUp. The operator also saw a dim left-to-right red wave. Color change is real; a *static* mode or independent per-key addressing was not demonstrated.
- `setkey` zero-filled the same payload except for one triple. Keys went dark and selecting vendor LED index 0 did **not** light Esc. This rejects the tool's positional-addressing assertion, **not** the vendor's physical key map.
- `sidelight` sent 18 RGB bytes immediately after command `0x08`; the side light did not change. It neither supplied the matrix handler's complete data nor proved that a report-0x09 command reaches the `0x08FA` internal staging path. This is not evidence that the side light is uncontrollable.
- The `regset` sweep sent short report-0x06 `'S'` frames. `0x5001` stages those frames and gates apply at a length threshold (`0x509A–0x50A8`); the short frames do not establish whether the target settings can be changed.
- The case light briefly looked white/ice-blue during a sequence of probes and later returned to rainbow. No isolated packet/response pair was captured for that transition. Its cause remains open.

## Next proof obligations (before another USB write)

1. Trace how USB feature data reaches `0x08FA` (or show that it cannot) and map the actual report prefix, length and apply gate. Do not equate the internal offset `8` with a host payload offset without that trace.
2. Trace `0x0379` through the PWM/LED output path; only then identify a physical-key index and build a single-key test with a complete, known-safe baseline for every other slot. Never use a zero-filled config image as a single-key test.
3. Trace the side/case output path backwards to its state and host-reachable writer. `0x0BBF` is a behavior flag (`0x2164`, `0x804B`, `0x8296`), not yet a side-zone RGB selector.
4. Keep report-0x09 command `0x04` forbidden. The ordinary Fn+PgUp keys-off state and the `0x04` reload failure are distinct; one being recoverable by an RGB write does not establish recovery from the other.
