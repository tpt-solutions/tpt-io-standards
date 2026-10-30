#!/usr/bin/env python3
import struct

data = open(r'C:\Programming\tpt-io-standards\crates\tpt-io-netcdf\tests\samples\chunked4.h5', 'rb').read()

def rdint(p, n):
    return int.from_bytes(data[p:p+n], 'little')

def parse_ohdr(addr):
    base = addr
    if data[base:base+4] != b'OHDR':
        return []
    ver = data[base+4]
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
    cont = []
    start = p
    remaining = chunk0 - (p - start)
    cnt = 0
    while cnt < nmess and p+6 <= len(data) and remaining >= 6:
        mt = rdint(p, 2)
        sz = rdint(p+2, 2)
        mf = data[p+4]
        p += 6
        body = data[p:p+sz]
        if mt == 0x0010:
            q = 0
            while q + 16 <= len(body):
                cont.append(rdint(q, 8))
                q += 16
        else:
            msgs.append((mt, sz, mf, body))
        cnt += 1
        p += sz
        if p % 8: p += 8 - (p % 8)
        remaining = chunk0 - (p - start)
    # follow continuations
    while p+6 <= len(data):
        if p+4 <= len(data) and data[p:p+4] == b'OHDR':
            p += 5
            flags = data[p]; p += 1
            c0w = 2 if flags & 0x01 else 1
            cnw = 2 if flags & 0x02 else 1
            chunk0 = rdint(p, c0w); p += c0w
            _cn = rdint(p, cnw); p += cnw
            nm = rdint(p, 2); p += 2
            _hs = rdint(p, 4); p += 4
            if flags & 0x08: p += 2
            if flags & 0x10: p += 4
            start = p; remaining = chunk0 - (p - start)
            cc = 0
            while cc < nm and p+6 <= len(data) and remaining >= 6:
                mt = rdint(p, 2)
                sz = rdint(p+2, 2)
                mf = data[p+4]
                p += 6
                body = data[p:p+sz]
                if mt == 0x0010:
                    q = 0
                    while q+16 <= len(body):
                        cont.append(rdint(q, 8)); q += 16
                else:
                    msgs.append((mt, sz, mf, body))
                cc += 1
                p += sz
                if p % 8: p += 8 - (p % 8)
                remaining = chunk0 - (p - start)
            continue
        break
    return msgs

# Find all datasets by parsing all OHDRs
for addr in [0x30, 0xb3]:
    msgs = parse_ohdr(addr)
    print("OHDR", hex(addr), "messages", len(msgs))
    for mt, sz, mf, body in msgs:
        print("  mt=0x%04X sz=%d shared=%s body=%s" % (mt, sz, bool(mf & 0x02), body[:20].hex()))
        if mt == 0x0008:
            print("    class", body[1], "ndims", body[2], "body", body.hex())
        if mt == 0x000A or mt == 0x000B:
            print("    filter body", body.hex())
