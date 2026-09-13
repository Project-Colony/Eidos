"""Regenerate the independent synthetic BSDIFF40 fixture with Python stdlib."""
from pathlib import Path
import bz2
import struct

# abc -> axcd!: one nonzero delta and two bytes in the extra stream.
control = bz2.compress(struct.pack("<QQQ", 3, 2, 0))
diff = bz2.compress(bytes([0, ord("x") - ord("b"), 0]))
extra = bz2.compress(b"d!")
patch = b"BSDIFF40" + struct.pack("<QQQ", len(control), len(diff), 5) + control + diff + extra
Path(__file__).with_name("tiny.bsdiff").write_bytes(patch)

# Minimal x86_64 PE with one RT_VERSION resource, independent of inspect_pe.
image = bytearray(0x600)
def u16(at, value):
    struct.pack_into("<H", image, at, value)
def u32(at, value):
    struct.pack_into("<I", image, at, value)
image[:2] = b"MZ"
u32(0x3c, 0x80)
image[0x80:0x84] = b"PE\0\0"
u16(0x84, 0x8664)
u16(0x86, 1)
u16(0x94, 240)
u16(0x98, 0x20b)
u32(0x98 + 108, 16)
u32(0x98 + 128, 0x1000)
u32(0x98 + 132, 0x200)
image[0x188:0x18e] = b".rsrc\0"
for at, value in [(0x190, 0x400), (0x194, 0x1000), (0x198, 0x400), (0x19c, 0x200)]:
    u32(at, value)
for at, identifier, target in [(0x200, 16, 24), (0x218, 1, 48), (0x230, 1033, 72)]:
    u16(at + 14, 1)
    u32(at + 16, identifier)
    u32(at + 20, target | (0x80000000 if at != 0x230 else 0))
u32(0x248, 0x1100)
u32(0x24c, 92)
u16(0x300, 92)
u16(0x302, 52)
image[0x306:0x326] = "VS_VERSION_INFO\0".encode("utf-16le")
for at, value in [(0x328, 0xfeef04bd), (0x32c, 0x10000), (0x330, 0x00010007), (0x334, 104 << 16)]:
    u32(at, value)
Path(__file__).with_name("runtime.pe").write_bytes(image)
