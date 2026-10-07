# Export strokes whose table lookup was confirmed exactly (speed, elevation AND frames) by traj_inverse2:
# table_file,kind,hit_x,hit_y,hit_z,target_x,target_z,vel_x,vel_y,vel_z,curve_frames
import sys, glob, struct, re, subprocess, numpy as np
out=open(sys.argv[2],'w')
lines=subprocess.run(['python3','tools/traj_inverse2.py',sys.argv[1]],capture_output=True,text=True).stdout.splitlines()
files={f.split('/')[-1][:-4]:f for f in glob.glob('xb/TRAJ/*/data/hatsuyama/traj/*.dat')}
for l in lines:
    m=re.match(r'(\d{3}) kind (\d) spin \S+ err (\S+) (\S+)\s+target \(\s*(\S+),\s*(\S+)\) frames (\d+) tbl (\d+)',l)
    if not m or float(m[3])>1e-4 or int(m[8])+1!=int(m[7]): continue
    o=open(f'{sys.argv[1]}/shot_{m[1]}.ball','rb').read()[4:4+0x290]
    pos=struct.unpack_from('<3f',o,0xe0); v=struct.unpack_from('<3f',o,0x130)
    out.write(",".join([files[m[4]].replace('xb/',''),m[2]]+[repr(float(x)) for x in (*pos,float(m[5]),float(m[6]),*v)]+[m[7]])+"\n")
