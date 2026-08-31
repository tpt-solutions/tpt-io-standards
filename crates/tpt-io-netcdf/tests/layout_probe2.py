import struct
data=open('crates/tpt-io-netcdf/tests/samples/chunked4.h5','rb').read()
i=data.find(b'TREE')
known_off=[8760,8782,8805,8826]
known_size=[22,23,21,24]  # aligned with known_off order
def get(buf, sz):
    return struct.unpack('<Q',buf[:8].ljust(8,b'\x00'))[0] if sz==8 else struct.unpack('<I',buf[:4])[0]
for offsz in (4,8):
    for lensz in (4,8):
        for coordsz in (4,8):
            for H in range(8, 80, 4):
                esz=offsz+lensz+4+2*coordsz
                if H+4*esz > len(data)-i: continue
                p=i+H
                res=[]
                ok=True
                for k in range(4):
                    ent=data[p:p+esz]
                    if len(ent)<esz: ok=False;break
                    off=get(ent[0:offsz],offsz)
                    size=get(ent[offsz:offsz+lensz],lensz)
                    fmask=struct.unpack('<I',ent[offsz+lensz:offsz+lensz+4])[0]
                    c0=get(ent[offsz+lensz+4:offsz+lensz+4+coordsz],coordsz)
                    c1=get(ent[offsz+lensz+4+coordsz:offsz+lensz+4+2*coordsz],coordsz)
                    res.append((off,size,fmask,(c0,c1)))
                    p+=esz
                if ok:
                    offs=[r[0] for r in res]
                    sizes=[r[1] for r in res]
                    if offs==known_off and sizes==known_size:
                        print('MATCH offsz',offsz,'lensz',lensz,'coordsz',coordsz,'HDR',H,'esz',esz)
                        for r in res: print('   ',r)
