# Shot table lookup as decoded from the game; inverse-checked against captured shots.
import numpy as np, struct, glob, os
f32=np.float32
ANG=f32(0.0007669904); SPD=f32(0.0009765625)
def load_table(path):
    return np.frombuffer(open(path,'rb').read(),'<u4')
def cell(t,i):
    w=int(t[min(i,len(t)-1)]); lo=w&0xfff  # game reads past the table at the top edge; clamped here; lo= lo-0x1000 if lo&0x800 else lo
    mid=(w>>12)&0xfff; mid= mid-0x1000 if mid&0x800 else mid
    return f32(lo)*ANG, f32(mid)*SPD, (w>>24)&0xff
def axis(v, lo, hi, n=16):
    u=f32(-(-(v)-lo))/f32(-(hi-lo)) if False else None
def norm(num, den):
    u=f32(num)/f32(den); u=min(max(u,f32(0)),f32(1)); s=f32(15)*u; i=int(s); fr=s-f32(i)
    if i==15: fr=f32(1)
    return i, fr
def lookup(t, fx, ix, fy, iy, fz, iz):
    """trilinear (x fastest, then y rows of 16, then z planes of 256); reads idx+1 even at the edge like the game"""
    def g(x,y,z): return cell(t, x + 16*y + 256*z)
    def lerp(a,b,u): return u*(b-a)+a
    out=[]
    for k in range(3):
        c=[ [g(ix,iy,iz)[k], g(ix+1,iy,iz)[k]], [g(ix,iy+1,iz)[k], g(ix+1,iy+1,iz)[k]] ]
        c2=[ [g(ix,iy,iz+1)[k], g(ix+1,iy,iz+1)[k]], [g(ix,iy+1,iz+1)[k], g(ix+1,iy+1,iz+1)[k]] ]
        if k<2:
            a=lerp(lerp(c[0][0],c[0][1],fx), lerp(c[1][0],c[1][1],fx), fy)
            b=lerp(lerp(c2[0][0],c2[0][1],fx), lerp(c2[1][0],c2[1][1],fx), fy)
            out.append(lerp(a,b,fz))
        else:
            a=int(lerp(f32(c[0][0]),f32(c[0][1]),fx)); bb=int(lerp(f32(c[1][0]),f32(c[1][1]),fx))
            lo=int(fy*f32(bb-a)+f32(a))
            a2=int(lerp(f32(c2[0][0]),f32(c2[0][1]),fx)); b2=int(lerp(f32(c2[1][0]),f32(c2[1][1]),fx))
            hi=int(fy*f32(b2-a2)+f32(a2))
            out.append(int(fz*f32(hi-lo)+f32(lo)))
    return out  # elevation(rad), speed(m/frame), frames
