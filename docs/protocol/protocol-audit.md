# Protocol audit: report 0x09 and the RGB matrix

This audit uses `fw/k75_full.bin` (8051 CODE offsets below), the live HID descriptor, and the operator's observations. Firmware traces establish a host-reachable RAM-matrix command; live comparisons identify **slot 0 as Esc and slot 63 as the UK-layout semicolon key**. Other slot positions and firmware-persistent mode remain unverified. Transport acceptance and a feature-report echo alone are not visual confirmation.

## Receive path and host layout

1. The live interface-1 HID descriptor advertises feature report ID `0x09` with `0x0207` data bytes (519, plus the report ID). At `0x8906–0x894A`, firmware checks XDATA `0x1149=0x09`, `0x114A=0x03`, `0x114B=0x01`, sets `0x1150=0x0B`, clears the cursor at `0x0B0D:0x0B0E`, and copies the requested transfer length from `0x114D:0x114E` into `0x0B0F:0x0B10`. The `0x09/0x03/0x01` comparison selects the feature-report-0x09 path for interface 1; `0x1150` is a **transfer state**, not host payload byte `0x50`.
2. The USB service routine `0x8765–0x87F1` sees state `0x0B` and jumps to `0x7253` with XDATA source `0x1100`. `0x7253` transfers up to eight bytes per iteration through `0xB0A8`; the latter reads `source + index` at `0xB0C5–0xB0D0`, writes `0x08FA + cursor` at `0xB0D4–0xB0E5`, and advances the cursor. `0x7253` subtracts eight from the remaining length and, on completion, writes `1` to `0x0DAE` at `0x72E1–0x72E9`.
3. The dispatcher at `0x7A72–0x7AAE` requires `0x0DAE=1`, reads the command at `0x08FB`, and routes command `0x08` to `0x7B35`. That branch calls `0x7108` with base `0x08FA` and source offset `8`. Thus the report bytes staged at offsets **8..385** supply the 126 RGB triples. No separate `0x0B` apply or flash-save command is part of this path.
4. Read-only `GetFeature(0x09)` returned `[09, 0B, 00, 00, 00, 00, 00, 00]` before testing (the last earlier probe ended with `0x0B`). A complete report beginning `[09, 08, 00, 00, 00, 00, 00, 00, FF, 00, 00, ...]` was sent using `matrix08 baseline --send`; transport accepted it and subsequent readback returned `[09, 08, 00, 00, 00, 00, 00, 00]`. This corroborates the staged header and command byte at complete report offset 1. Re-sent while the keys were dark and the case light was rainbow: **all keys turned red**, while the case remained rainbow.
5. `matrix08 slot 63 --send` differs from the complete red baseline in only the red and green channels at report offsets `197..199`. On two observed sends, the operator saw **one key—the UK-layout `;` key next to L—turn steady green, with the other keys red**. The key lights returned to off after approximately two seconds. A separate slot-0 test at offsets `8..10` turned **Esc** green against red keys. Both keys agree with the vendor `KB.ini` LED-ID table at these indices; **the rest of that table is not yet visually verified as host slots**. The case side light remained rainbow. The operator later noted that its rainbow looked static during a slot-0 test, but could not attribute that to this command.
6. A bounded run of 16 identical slot-63 reports at 500 ms intervals kept the red keys and green `;` lit throughout the sends; the key lights went off after sends stopped. The operator confirmed that the case side stayed rainbow. This shows host streaming can sustain the otherwise transient display, **not** that a persistent/static firmware mode or case-light control has been identified.

The report generator is `tools/hidra_probe/src/bin/matrix08.rs`. It defaults to an offline dry run; `--send` is explicit and `--repeat 1..20` makes a finite series of the *same* RAM-only frame at 500 ms intervals. It fills every matrix slot and never sends save, reload, ISP, or any other command. It is a controlled test instrument, **not** a complete UK ISO mapping or a firmware static-mode selector. The earlier `sidelight` and `setkey` probes remain disabled.

## Internal matrix and renderer

- `0x7108`: six inner iterations (`0x711D–0x7127`) and 21 outer iterations (`0x723E–0x724C`). Source `0x08FA + 8 + row×18 + column×3 + channel`, destination `0x0379 + row×18 + column×3 + channel`. The 378 source bytes are XDATA `0x0902..0x0A7B`; the destination is `0x0379..0x04F2`. It marks `0x0E34=0x5A` on completion. **These are RGB slots, not a UK ISO key-to-slot map.**
- `0x5AF8–0x5B03` consumes the `0x0E34=0x5A` marker. Under one mode (`0x0F54=1`) it reads the RGB matrix to construct another table beginning at `0x05F4`; other modes take a branch through `0x4A22`. At `0x5BEC–0x5C06` it clears the marker, sets a renderer flag, and loads `0x0F59=0xFA`. The scheduler at `0x7781–0x7792` continues rendering while that counter is nonzero; `0x35BB–0x35CF` decrements it and changes the effect code when it reaches zero. This offers a firmware explanation for the observed ~two-second display; the precise timer period and a persistent-mode command are not yet established. Another renderer branch at `0x2E8A–0x2ED7` reads the matrix and writes to `0x08FA`; this internal reuse of the command-buffer address does not reverse the USB receive trace.
- `0xA65B–0xA66B` and `0x6AEE` implement the **opposite** path: `0x08FA..0x0901` to `0x1108..0x110F` for the USB response. This explains why feature readback can corroborate the staged header; by itself, it would not have proved the receive path.
- The separate `0x0A` command branch at `0x7B2A–0x7B33` calls `0x9308`, which uses a flash-buffer operation, **not** the positional matrix writer. Do not reinterpret earlier `0x0A` + `0x0B` color changes as per-key control.

`python3 analysis/verify_matrix_path.py` checks instruction signatures and the address arithmetic offline. It cannot establish live transport, renderer conditions, physical key positions, or case-light routing.

## Case/side light (separate animation generator)

The case light is **not** the 126-slot key matrix. It is a self-contained effect engine on its own XDATA registers `0x0F99..0x0F9F`:

- `0x0F99` = case colour index into CODE palette `0xB459`/`0xB463`; `0x0F9B` = effect sub-state; `0x0F9D` = step/direction counter; `0x0F9E` = current case colour byte; `0x0F9F` = palette index into `0xB46D`.
- The case effect loop is `0x0B66–0x0E70`. The case renderer at `0x4D80` copies `0x0F99/0x0F9A/0x0F9C/0x0F9F` into a `0x0DBF..0x0DC3` staging block and `0x0BBF` into `0x0DD9`, then serialises out via `lcall 0xda4f` (the LED transmit) from base `0x0DB2`.
- Every writer of `0x0F99..0x0F9F` is the firmware effect loop (`0xB66–0xE70`), init (`0x3701–0x3835`), `0x4931`, or the `0x6245/0x6233` region — **none** in the report-0x09 or config-write handler ranges (`0x72xx–0x93xx`). There is **no live-USB-frame path into the case colour**.
- The only host-reachable lever is `0x0BBF`, seeded by the config-load path `0x7F15` from CODE `0xA427` (profile byte `+0x0F`). The case light is therefore **config/profile-driven**, which explains why a config *reload* (forbidden `0x04`) visibly changed it to white/ice-blue while the `0x08` key tests left it rainbow.

To change the case light, write profile byte `+0x0F` (feeds `0x0BBF`) through the config path (`0x0A` + `0x0B`), **not** through the key-matrix `0x08` command. This `0x0BBF → case colour` link is a firmware hypothesis and has **not** been tested live.

## Second command channel (control transfers)

A second, independent host path exists on the USB control endpoint, dispatched by
wValue rather than by a feature-report command byte. The SETUP handler at
`0x14A9` reads XDATA `0x114A` (wValue **high**) and branches:

- `0x114A == 0x01` → writes magic `0x0FA3:0x0FA4 = "AH"`, reads a length from
  `0x0F6C`, stages a pointer into `0x1151..0x1153`, `ljmp 0x17FF`.
- `0x114A == 0x02` → writes magic `"AZ"`, reads `0x0F6D:0x0F6E`, stages into
  `0x1151..`, continues.

`0x17FF` stores the staged 16-bit pointer in `0x1151:0x1152` and `lcall 0x906D`,
which walks the **`0x05F4`-based effect table** (0x24-byte records) doing
pointer arithmetic, using `0x1150` as a status/state register and treating the
`0x0FA3:0x0FA4` magic bytes as a running 16-bit pointer. `wLength` is
`0x114D:0x114E`.

This control-transfer channel is the **likely carrier for effect selection**
(the `0x0F3F`/`0x0F83` persistence path), but it is complex effect-table-walk
plumbing, not a single clean "set effect N" command. It is traced offline and
**not** verified live. It does not supersede the feature-report `0x08` matrix
path — the two are distinct.

## Visual evidence and outstanding tests

- A full-red `0x0A` payload followed by `0x0B` previously made the keys red after Fn+PgUp, with a dim left-to-right **red wave**. That is not proof of static mode or individual keys.
- The former `sidelight` wrote 18 bytes immediately after command `0x08`, rather than a complete matrix beginning at report offset 8. The side-light null result is not an independent case-zone test. During the correctly framed matrix baseline, single-slot test, and bounded stream, the operator saw the case light **remain in its default rainbow state**; the case output still needs its own backward trace.
- The full-red and green-slot tests verify individual key control, but only transiently. For physical mapping, keep a complete baseline in every report and observe one changed slot at a time. Investigate a persistent firmware mode separately; bounded streaming merely refreshes the transient effect.
- Report-0x09 command `0x04` remains forbidden: prior live testing left lights off without a known USB recovery. Fn+Esc recovered the device. Do not send ISP flash-erase `0x45` or flash firmware.
