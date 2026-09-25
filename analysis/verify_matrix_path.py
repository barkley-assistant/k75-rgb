"""Verify the firmware instructions behind the internal RGB-matrix layout.

This script reads the stock firmware only; it does not access the keyboard.
It validates the disassembly offsets used by docs/protocol-audit.md and
prints an address map for internal slots, not physical key positions.
"""

from pathlib import Path

FIRMWARE = Path(__file__).resolve().parents[1] / "fw" / "k75_full.bin"


def main() -> None:
    firmware = FIRMWARE.read_bytes()
    signatures = {
        0x6AFA: bytes.fromhex("90 08 fa e0 fd 0f"),
        0x711D: bytes.fromhex("90 0e ea e0 ff c3 94 06"),
        0x715A: bytes.fromhex("75 f0 12 eb a4 24 79"),
        0x71B6: bytes.fromhex("75 f0 12 eb a4 24 7a"),
        0x7212: bytes.fromhex("75 f0 12 eb a4 24 7b"),
        0x7245: bytes.fromhex("64 15"),
        0x724C: bytes.fromhex("90 0e 34 74 5a f0 22"),
        0x7B35: bytes.fromhex("7e 08 7f fa 7d 08 12 71 08"),
    }
    for offset, expected in signatures.items():
        actual = firmware[offset : offset + len(expected)]
        if actual != expected:
            raise RuntimeError(
                f"firmware mismatch at 0x{offset:04X}: "
                f"expected {expected.hex(' ')}, got {actual.hex(' ')}"
            )

    slots = [
        (row, column, 0x08FA + 8 + row * 18 + column * 3,
         0x0379 + row * 18 + column * 3)
        for row in range(21)
        for column in range(6)
    ]
    assert len(slots) == 126
    assert slots[0] == (0, 0, 0x0902, 0x0379)
    assert slots[-1] == (20, 5, 0x0A79, 0x04F0)
    print("Firmware signatures verified; internal matrix: 21 x 6 RGB slots")
    print("Source XDATA: 0x0902..0x0A7D; destination XDATA: 0x0379..0x04F2")
    print("Physical key mapping and host USB packet layout remain unverified")


if __name__ == "__main__":
    main()
