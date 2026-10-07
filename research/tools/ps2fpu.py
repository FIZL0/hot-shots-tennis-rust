# PS2 EE FPU arithmetic model: round toward zero, and the smaller addend is truncated to the larger's
# precision before adding (alignment truncation).
import struct, math
from fractions import Fraction as F
def bits(x): return struct.unpack('<I',struct.pack('<f',x))[0]
def fromb(b): return struct.unpack('<f',struct.pack('<I',b & 0xffffffff))[0]
def rtz(fr):
    if fr==0: return 0.0
    v=struct.unpack('<f',struct.pack('<f',float(fr)))[0]
    if abs(F(v))>abs(fr): v=fromb(bits(v)-1)
    elif abs(F(fromb(bits(v)+1)))<=abs(fr): v=fromb(bits(v)+1)
    return v
def trunc_to(fr, ulp):
    q=fr/ulp; q=math.floor(q) if q>=0 else -math.floor(-q)
    return F(q)*ulp
def add(a,b):
    if a==0: return b
    if b==0: return a
    ea=math.frexp(a)[1]; eb=math.frexp(b)[1]
    big,small,e=(a,b,ea) if ea>=eb else (b,a,eb)
    ulp=F(2)**(e-24)
    return rtz(F(big)+trunc_to(F(small),ulp))
def sub(a,b): return add(a,-b)
def mul(a,b): return rtz(F(a)*F(b))
def madd(acc,a,b): return add(acc, mul(a,b))
