import struct
data=open('crates/tpt-io-netcdf/tests/samples/chunked4.h5','rb').read()
i=data.find(b'TREE')
offsz=8; lensz=8; ndims=2
known_off=[8760,8782,8805,8826]
known_size=[22,23,21,24]
known_coords=[(0,0),(0,3),(2,0),(2,3)]
hdr=4+1+1+2+offsz+offsz
def get(buf, sz):
    if sz==8: return struct.unpack('<Q',buf)[0]
    return struct.unpack('<I',buf)[0]
for coordsz in (4,8):
    for order in ('os','so'):
        esz=offsz+lensz+4+ndims*coordsz
        if esz>64: continue
        p=i+hdr
        ok=True; res=[]
        for k in range(4):
            ent=data[p:p+esz]
            if len(ent)<esz: ok=False; break
            try:
                if order=='os':
                    off=get(ent[0:offsz],offsz)
                    size=get(ent[offsz:offsz+lensz],lensz)
                    fmask=struct.unpack('<I',ent[offsz+lensz:offsz+lensz+4])[0]
                    c=struct.unpack('<%dI'%ndims,ent[offsz+lensz+4:offsz+lensz+4+ndims*coordsz])
                else:
                    size=get(ent[0:lensz],lensz)
                    off=get(ent[lensz:lensz+offsz],offsz)
                    fmask=struct.unpack('<I',ent[lensz+offsz:lensz+offsz+4])[0]
                    c=struct.unpack('<%dI'%ndims,ent[lensz+offsz+4:lensz+offsz+4+ndims*coordsz])
            except struct.error:
                ok=False; break
            res.append((off,size,fmask,c))
            p+=esz
        if ok and all(r[0] in known_off for r in res):
            print('MATCH coordsz',coordsz,'order',order,'esz',esz)
            for r in res: print('   ',r)
