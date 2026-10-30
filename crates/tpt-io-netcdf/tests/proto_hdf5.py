#!/usr/bin/env python3
"""Prototype HDF5/NetCDF-4 decoder to validate the message layout against the
real h5py-generated fixtures, before porting the algorithm to Rust.

Strategy: object headers begin with b"OHDR" (v2). The exact prefix length
varies, so we locate the first message by finding the longest chain of
consecutive valid message headers (type known, size plausible, next header
valid). We then walk Link messages (0x0006) to enumerate child objects.
"""
import struct

data = open(r'C:\Programming\tpt-io-standards\crates\tpt-io-netcdf\tests\samples\example4.h5', 'rb').read()

KNOWN = {0x0001,0x0002,0x0003,0x0005,0x0006,0x0008,0x000A,0x000B,0x000C,
         0x0010,0x0012,0x0013,0x000D,0x0000,0x0004,0x0007,0x0009,0x000E,
         0x000F,0x0011,0x0014}

def u16(b, o): return struct.unpack('>H', b[o:o+2])[0]
def u32(b, o): return struct.unpack('>I', b[o:o+4])[0]
def u64(b, o): return struct.unpack('>Q', b[o:o+8])[0]
def u8(b, o): return b[o]

def find_first_message(start, maxscan=4000):
    # find longest chain of valid messages starting at some offset >= start+6
    best = None
    for p in range(start+6, start+6+40):  # prefix is small
        msgs = []
        off = p
        ok = True
        for _ in range(64):
            if off+8 > len(data): ok=False; break
            t = u16(data, off)
            sz = u16(data, off+2)
            if t not in KNOWN or sz > 1<<20 or off+8+sz > len(data):
                ok=False; break
            msgs.append((off, t, sz))
            nxt = off+8+sz
            off = nxt
            if off % 8: off += 8 - (off % 8)
        if ok and len(msgs) >= 2:
            if best is None or len(msgs) > len(best[2]):
                best = (p, start, msgs)
    return best

def parse_messages(start):
    r = find_first_message(start)
    if r is None:
        return []
    _, _, msgs = r
    return msgs

# locate all object headers
def find_object_headers():
    res = []
    i = 0
    while True:
        j = data.find(b'OHDR', i)
        if j < 0: break
        res.append(j)
        i = j+4
    return res

print("object headers at:", find_object_headers())

# Parse root group (first OHDR)
root = find_object_headers()[0]
msgs = parse_messages(root)
print("root first-msg scan found", len(msgs), "messages; first at", msgs[0][0] if msgs else None)
for (off, t, sz) in msgs[:12]:
    name = {0x0001:'dataspace',0x0002:'linkinfo',0x0003:'datatype',0x0005:'fill',
            0x0006:'link',0x0008:'layout',0x000A:'groupinfo',0x000B:'filter',
            0x000C:'attr',0x0010:'cont',0x0012:'attrinfo'}.get(t, hex(t))
    print(f"  off {off} {name} sz {sz}")
