# Key-matrix slot map (firmware 21×6 vs physical 16×6)

## Vendor LED index is column-major (idx = col×6 + row)

The official `KB.ini` `[KEY]` section numbers LEDs as `idx = column×6 + row`,
where the physical board is **16 columns × 6 rows = 96 positions** (81 real
keys + structural gaps + the extra "LED 95" Mute). Grouped by `idx//6`
(column) and `idx%6` (row):

```
col 0  (r0..r5): Esc, `, Tab, CapsLock, LShift, LCtrl
col 1  (r1..r5): 1, Q, A, ><, LWin
col 2  (r0..r5): F1, 2, W, S, Z, LAlt
col 3  (r0..r4): F2, 3, E, D, X
col 4  (r0..r4): F3, 4, R, F, C
col 5  (r0..r5): F4, 5, T, G, V, Space
col 6  (r0..r4): F5, 6, Y, H, B
col 7  (r0..r4): F6, 7, U, J, N
col 8  (r0..r5): F7, 8, I, K, M, RAlt
col 9  (r0..r5): F8, 9, O, L, ,, FN
col 10 (r0..r4): F9, 0, P, ;, .
col 11 (r0..r4): F10, -, [, ', /
col 12 (r0..r3): F11, =, ], \
col 13 (r0,r3..r5): F12, Enter, RShift, Left
col 14 (r0,r4..r5): Delete, Up, Down
col 15 (r0..r1,r3,r5): Mute, Home, PgDn, Right
```

(Every real key's vendor index is `col×6 + row`; the physical column and row
can be read straight off this grid.)

## Firmware matrix is 21×6, not 16×6

The firmware matrix writer `0x7108` and renderer `0x2dd6` both iterate
**21 outer × 6 inner**. The ingest copies 126 RGB triples (report offsets
8..385) into XDATA `0x0379..0x04F2` with an 18-byte (`0x12`) per-row stride.
The renderer re-emits them into the physical LED chain with a **6-wide** group
stride (`row × 6`), so the physical chain is **21 groups × 6 = 126 LEDs**.

## The 126-slot chain vs 96 physical key positions

- Physical key grid: 16 cols × 6 rows = 96 positions (81 keys + gaps).
- Firmware chain: 21 groups × 6 = 126 slots.
- **30 slots have no key** (126 − 96). These are unpopulated key positions
  and/or the case/side underglow LEDs, which are wired into the same serial
  chain but driven by the separate case engine (see
  [protocol-audit.md](../protocol/protocol-audit.md)) rather than the key matrix.

## What this means for mapping the remaining keys

The host matrix slot `N` (report offset `8 + N×3`) does **not** equal the
vendor LED index. Two slots are visually pinned:

- slot 0 → Esc (vendor idx 0)
- slot 63 → UK `;` (vendor idx 63)

Since both slot and vendor idx agree at 0 and 63, the leading hypothesis is
**host slot = vendor LED index** for the 82 real keys, with the vendor index
itself encoding `col×6 + row`. But this is only confirmed at two points and
must not be extrapolated. The 30 non-key slots (indices not in the vendor
table) are unmapped — they are either dead or the case-light chain.

## Next live test (needs the device, do NOT send unattended)

To complete the map, send one `matrix08 slot N --send` per candidate, changing
only slot N to green against a red baseline, and record which physical key
lights green. Do this for a spread of indices (e.g. 12, 24, 36, 91, 95) to
confirm or refute "slot = vendor LED index" across rows and columns, and to
find whether any non-key slot lights the case.