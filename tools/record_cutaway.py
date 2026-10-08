#!/usr/bin/env python3
"""Record the post-point cut-away camera from slot 5: every frame its shot is on, u32 vsync + gm 0x100 + camera
handler *(gm+0xbc) 0x100 + the shot camera *(handler+0x64) 0x3100 + the scene reference frames *(gm+0xb8)+0xd130
(0x60 x 0x40) + scene +0x10e00 0x200. With --players each sample also gets the point globals 0x423040 0x80, the
shot-pick counters 0x427b58 0x18, gm+0x340 0x40 and per player (4): head bone *(pl+0x17f4) 0x40, spine2 bone
*(pl+0x17fc) 0x40, pl+0x17e0 0x10, pl+0x3d40 0x80, pl+0x3db0 0x10, pl+0x12b8 0x8, anim object *(pl+0x54) 0x40, its
clip's length (+0x2c) 8. Usage: record_cutaway.py <out.bin> <frames> [--players]"""
import sys,time,struct
sys.path.insert(0,'tools')
from pine import Pine
p=Pine(); p.load_state(5); time.sleep(1)
out=open(sys.argv[1],'wb'); n=0; want=int(sys.argv[2])
VS=0x1d5780
last=p.read32(VS)
while n<want:
    v=p.read32(VS)
    if v==last: continue
    last=v
    gm=p.read32(0x422f80)
    if not gm: continue
    ch=p.read32(gm+0xbc)
    if p.read8(ch+0x51) in (0,0xff): continue
    cam=p.read32(ch+0x64); sc=p.read32(gm+0xb8)
    rec=struct.pack('<I',v)+p.read_block(gm,0x100)+p.read_block(ch,0x100)+p.read_block(cam,0x3100)+p.read_block(sc+0xd130,0x60*0x40)+p.read_block(sc+0x10e00,0x200)
    if '--players' in sys.argv:
        r=[(0x423040,0x80),(0x427b58,0x18),(gm+0x340,0x40)]
        for i in range(4):
            pl=p.read32(gm+0xa8+4*i); a=p.read32(pl+0x54)
            r+=[(p.read32(pl+0x17f4),0x40),(p.read32(pl+0x17fc),0x40),(pl+0x17e0,0x10),(pl+0x3d40,0x80),(pl+0x3db0,0x10),(pl+0x12b8,8),(a,0x40),(p.read32(a+0x24)+0x28,8)]
        rec+=p.read_regions(r)
    out.write(rec); n+=1
print('done',n)
