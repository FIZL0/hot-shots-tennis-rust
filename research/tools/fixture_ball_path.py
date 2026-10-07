# Export the stored ball path from a savestate RAM image as CSV: frame,px,py,pz,vx,vy,vz (game Y-down space)
import numpy as np, struct, sys
d=open(sys.argv[1],'rb').read(); u=lambda a: struct.unpack_from('<I',d,a)[0]
gm=u(0x422f80); rec=u(gm+0xa4); arr,n=u(rec+0x50),u(rec+0x54)
with open(sys.argv[2],'w') as o:
    for i in range(n):
        e=np.frombuffer(d[arr+i*0x30:arr+i*0x30+0x30],'<f4')
        o.write(",".join([str(i)]+[repr(float(x)) for x in (e[0],-e[1],e[2],e[4],-e[5],e[6])])+"\n")
