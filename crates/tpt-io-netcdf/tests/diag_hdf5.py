#!/usr/bin/env python3
import struct

data = open(r'C:\Programming\tpt-io-standards\crates\tpt-io-netcdf\tests\samples\chunked4.h5', 'rb').read()

def rdint(p, n):
    return int.from_bytes(data[p:p+n], 'little')

offsz = data[9]
lensz = data[10]
print("offsz", offsz, "lensz", lensz, "len", len(data))

def find_all(sig):
    res = []
    i = 0
    while True:
        j = data.find(sig, i)
        if j < 0: break
        res.append(j); i = j + len(sig)
    return res

ohdrs = find_all(b'OHDR')
print("OHDR addrs", [hex(o) for o in ohdrs])

def parse_object_header(addr):
    base = addr
    assert data[base:base+4] == b'OHDR', data[base:base+4]
    v = data[base+4]
    assert v == 2, v
    p = base + 5
    flags = data[p]; p += 1
    c0w = 2 if flags & 0x01 else 1
    cnw = 2 if flags & 0x02 else 1
    chunk0 = rdint(p, c0w); p += c0w
    chunkn = rdint(p, cnw); p += cnw
    nmess = rdint(p, 2); p += 2
    hsize = rdint(p, 4); p += 4
    if flags & 0x08: p += 2
    if flags & 0x10: p += 4
    msgs = []
    start = p
    remaining = chunk0 - (p - start)
    cnt = 0
    while cnt < nmess and p+6 <= len(data) and remaining >= 6:
        mt = rdint(p, 2)
        sz = rdint(p+2, 2)
        mf = data[p+4]
        p += 6
        body = data[p:p+sz]
        msgs.append((mt, sz, mf, body, p))
        cnt += 1
        p += sz
        if p % 8: p += 8 - (p % 8)
        remaining = chunk0 - (p - start)
    return msgs

for oh in ohdrs:
    try:
        msgs = parse_object_header(oh)
    except Exception as e:
        print("OHDR", hex(oh), "parse error:", e)
        continue
    print("=== OHDR", hex(oh), "messages:", len(msgs))
    for mt, sz, mf, body, p in msgs:
        print("   mt=0x%04X sz=%d shared=%s body[:16]=%s" % (mt, sz, bool(mf & 0x02), body[:16].hex()))

# Now directly locate chunked layout: scan all OHDRs for mt 0x0008 with class 2
for oh in ohdrs:
    try:
        msgs = parse_object_header(oh)
    except Exception:
        continue
    for mt, sz, mf, body, p in msgs:
        if mt == 0x0008:
            lclass = body[1]
            print("layout at", hex(oh), "body hex:", body.hex())
            if lclass == 2:
                ndims = body[2]
                cdims = struct.unpack('<%dI' % ndims, body[4:4+ndims*4])
                print("  ndims", ndims, "chunk dims", cdims)
                pp = 4 + ndims*4
                bufsize = struct.unpack('<I', body[pp:pp+4])[0]; pp += 4
                print("  chunk buf size", bufsize)
                btree_addr = rdint(pp, offsz)
                print("  btree addr", hex(btree_addr))
                ba = btree_addr
                print("  TREE sig:", data[ba:ba+4])
                ntype = data[ba+4]
                level = data[ba+5]
                nused = struct.unpack('<H', data[ba+6:ba+8])[0]
                print("  BTREE type", ntype, "level", level, "nused", nused)
                ep = ba + 8
                lsib = rdint(ep, offsz); rsib = rdint(ep+offsz, offsz)
                print("  left", hex(lsib), "right", hex(rsib))
                p2 = ba + 8 + 2*offsz
                for i in range(nused):
                    off = rdint(p2, offsz); p2 += offsz
                    size = rdint(p2, lensz); p2 += lensz
                    fmask = struct.unpack('<I', data[p2:p2+4])[0]; p2 += 4
                    coords = struct.unpack('<%dI'%ndims, data[p2:p2+ndims*4]); p2 += ndims*4
                    print("  chunk", i, "addr", hex(off), "size", size, "fmask", fmask, "coords", coords, "head", data[off:off+12].hex())
