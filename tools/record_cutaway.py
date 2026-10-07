#!/usr/bin/env python3
"""Record the post-point cut-away camera from slot 5: every frame its shot is on, u32 vsync + gm 0x100 + camera
handler *(gm+0xbc) 0x100 + the shot camera *(handler+0x64) 0x3100 + the scene reference frames *(gm+0xb8)+0xd130
(0x60 x 0x40) + scene +0x10e00 0x200. Usage: record_cutaway.py <out.bin> <frames>"""
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
    out.write(rec); n+=1
print('done',n)
