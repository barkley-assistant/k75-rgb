# Wireless (2.4 GHz) path — dongle forensics

Read-only enumeration of the 2.4 GHz dongle, 2026-09-26. No writes were
sent to any device.

## Identity

| Field | Value |
|---|---|
| USB VID:PID | **258A:0150** |
| Product string | "Gaming KB" |
| Manufacturer | SINOWEALTH |
| bcdDevice | 0x0209 |
| Note | KB.ini claims dongle VID `0x3554` PID `0x0150` — the real device reuses the vendor ID **258A**. The KB.ini value is a red herring. |

The dongle enumerates with two interfaces, mirroring the wired layout:

| Interface | Wired 258A:019D | Dongle 258A:0150 |
|---|---|---|
| 0 | keyboard (boot kb + system + consumer) | identical |
| 1 | vendor: 0x01..0x09 | vendor: 0x01..0x07, **plus 0x0A/0x0B/0x13** |

## Report surface comparison (interface 1)

| Report | Wired | Dongle |
|---|---|---|
| 0x01 | IN 1 B — system | IN 1 B |
| 0x02 | IN 2 B — consumer | IN 2 B |
| 0x03 | IN 3 B — vendor FF02 | IN 3 B |
| 0x04 | IN 13 B — keyboard+consumer | IN 13 B |
| 0x05 | FEAT 5 B — FF00 | FEAT 5 B |
| 0x06 | FEAT 1031 B — FF00 (ISP/data channel) | FEAT 1031 B |
| 0x07 | IN 4 B | IN 4 B |
| **0x09** | **FEAT 519 B + IN 7 B — the lighting channel** | **absent** |
| 0x0A | — | FEAT 7 B — FF01 |
| 0x0B | — | IN 2 B |
| 0x13 | — | IN 19 B + OUT 19 B — FF02 radio control channel |

## Conclusion

**The dongle does not expose the 520-byte lighting report (`0x09`).**
Live RGB drive over 2.4 GHz is therefore not possible through the HID
surface as we know it — the wireless side has a 19-byte radio-control
report (0x13) and a small 7-byte feature report (0x0A), neither of which
can carry a matrix frame.

This is consistent with, and explains, the 2026-09-24 verification: the
flash save (`0x0a`/`0x0b`/`0x06`) writes the matrix into the keyboard's
own flash; in 2.4 GHz mode the keyboard **replays the saved pattern
itself**, with no host involvement. 2.4 GHz = save-then-replay, not live
drive.

### What this means for the tool

- **Supported**: configure + save over wired, unplug, use wirelessly —
  the user's original goal, and it needs no wireless protocol work.
- **Not possible without new RE**: live per-frame lighting over 2.4 GHz.
  It would require decoding the 19-byte `0x13` radio-frame wrapper
  between dongle MCU and keyboard MCU — a separate protocol layer with
  no host-side HID ingress.

## Files

- `analysis/dongle-2026-09-26/desc-hidraw{0..3}.bin` — raw report
  descriptors (wired 0/1, dongle 0/1).