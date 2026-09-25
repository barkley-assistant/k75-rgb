"""Check stock-firmware signatures for report ingress and the RGB matrix.

This is an offline guard against quoting the wrong firmware/offsets, not an
emulator or a substitute for observing USB receive and physical LEDs.
"""

from pathlib import Path

FIRMWARE = Path(__file__).resolve().parents[1] / "fw" / "k75_full.bin"


def main() -> None:
    firmware = FIRMWARE.read_bytes()
    signatures = {
        # Feature-report 0x09 setup, staging, completion and dispatch.
        0x87D5: bytes.fromhex("90 11 50 e0 b4 0b 16"),
        0x87EB: bytes.fromhex("7f 00 7e 11 02 72 53"),
        0x8906: bytes.fromhex("90 11 49 e0 fd 64 09"),
        0x891B: bytes.fromhex("90 11 50 74 0b f0"),
        0x8921: bytes.fromhex("e4 90 0b 0d f0 a3 f0"),
        0x8934: bytes.fromhex("90 0b 0f ee f0 a3 ef f0"),
        0x7291: bytes.fromhex("aa 06 f9 7b 01 7f 08 12 b0 a8"),
        0xB0D4: bytes.fromhex("90 0b 0d e0 fc a3 e0 24 fa"),
        0xB0DF: bytes.fromhex("74 08 3c f5 83 ee f0"),
        0x72E4: bytes.fromhex("90 0d ae 74 01 f0"),
        0x7A83: bytes.fromhex("90 08 fb e0 24 fd"),
        0x7AAE: bytes.fromhex("02 7b 35"),
        0x5AF8: bytes.fromhex("90 0e 34 e0 64 5a"),
        0x5C01: bytes.fromhex("90 0f 59 74 fa f0"),
        0x7781: bytes.fromhex("90 0e 34 e0 64 5a"),
        0x35BB: bytes.fromhex("90 0f 59 e0 60 0f 14 f0"),
        # GET response reads the matrix/command scratch in the opposite direction.
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
    assert (slots[-1][2] + 2, slots[-1][3] + 2) == (0x0A7B, 0x04F2)
    # Each staged host byte is copied to XDATA[0x08FA + report_offset].
    assert slots[0][2] - 0x08FA == 8
    assert slots[-1][2] - 0x08FA == 383
    print("Firmware signatures verified; report 0x09 receive -> 0x08FA staging -> command 0x08 matrix")
    print("Matrix: 21 x 6 RGB slots; report offsets 8..385")
    print("Source XDATA: 0x0902..0x0A7B; destination XDATA: 0x0379..0x04F2")
    print("Offline checker alone cannot verify physical keys or visible output; see docs/protocol-audit.md")


if __name__ == "__main__":
    main()
