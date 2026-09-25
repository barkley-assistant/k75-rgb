#!/usr/bin/env python3
"""build_tests.py — regenerate every config image for test batch 2.

All images derive from analysis/config-default-72.bin (factory-verbatim
72-byte profile) by patching single bytes at instruction-verified
config-load offsets:

  +0x0E -> 0x0CC8  wave SPEED param (0x25 slowest .. 0x55 fastest)
  +0x1B -> 0x0F82  effect param -> [0x0F22] = value - 1 (flag byte source)
  +0x1F -> 0x0F54  the real MODE register (0x25/0x35/0x45/0x55 = Fn+Tab cycle)
  +0x0F -> 0x0BBF  behavior-switch flag (side-light zone prime suspect)
"""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT = os.path.join(HERE, "config-default-72.bin")

def build(name, patches):
    data = bytearray(open(DEFAULT, "rb").read())
    assert len(data) == 72, f"config-default-72.bin is {len(data)} bytes, expected 72"
    for off, val in patches.items():
        assert 0 <= off < 72
        data[off] = val
    path = os.path.join(HERE, name)
    open(path, "wb").write(data)
    print(f"  {name}: " + " ".join(f"+{off:02X}={val:02X}" for off, val in patches.items()))

print("Pattern A — mode sweep (+0x1F = Fn+Tab register 0x0F54)")
build("t2-a1-mode25-72.bin", {0x1F: 0x25})
build("t2-a2-mode35-72.bin", {0x1F: 0x35})
build("t2-a3-mode45-72.bin", {0x1F: 0x45})
build("t2-a4-mode55-72.bin", {0x1F: 0x55})

print("Pattern B — effect-flag sweep (+0x1B -> [0x0F22] = val-1)")
for val in (0x00, 0x03, 0x05, 0x0B, 0x0D, 0x11, 0x13):
    build(f"t2-b{val:02x}-eff-72.bin", {0x1B: val})

print("Pattern C — speed sweep (+0x0E -> 0x0CC8)")
for val in (0x25, 0x35, 0x45, 0x55):
    build(f"t2-c{val:02x}-speed-72.bin", {0x0E: val})

print("Pattern S — side-light sweep (+0x0F -> 0x0BBF behavior flag)")
for val in (0x01, 0x02, 0x55, 0xFF):
    build(f"t2-s{val:02x}-side-72.bin", {0x0F: val})

print("done")