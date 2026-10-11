# Checks the court backdrop placement (gs::backdrop) against RAM dumps: for each court whose envir is in the dump,
# every drawn sky/clo/acc/bg matrix should sit at main eye + (0, lift, 0) with rotY(yaw). Run from context/:
#   python3 ../research/backdrop_check.py ram/s05.bin ram/npc_c02.bin ...
import struct,math,glob,os,numpy as np,sys
f=lambda d,o:struct.unpack_from('<f',d,o)[0]
for dump in sys.argv[1:]:
  r=np.frombuffer(open(dump,'rb').read()[:32<<20],dtype='<f4'); raw=r.tobytes()
  eye=r[0x1e7d20//4:0x1e7d20//4+3]
  for p in sorted(glob.glob('p17k/xb/COURT/*/CMN.XB/data/court/plant/envir_c??.dat')):
    env=open(p,'rb').read(); n=p.split('/')[3]
    if env[0x450:0x470] not in raw: continue
    hp=glob.glob(f'p17k/xb/COURT/{n}/GRD01.XB/data/court/plant/envir_c{n}_h01.dat')
    h=open(hp[0],'rb').read() if hp else None
    for i in range(4):
      turn,lift=(f(h,0x58),f(h,0x10+4*i)) if h else (0.0,370.0)
      y=f(env,0x450+4*i)-turn*0.017453292
      if y>math.pi:y-=2*math.pi
      elif y<-math.pi:y+=2*math.pi
      t=eye+np.array([0,lift,0],dtype='<f4')
      k=[j for j in np.where((abs(r[12:-2]-t[0])<1e-4)&(abs(r[13:-1]-t[1])<1e-3)&(abs(r[14:]-t[2])<1e-4))[0]]
      ok=[j for j in k if abs(r[j]-math.cos(y))<1e-3 and abs(r[j+8]-math.sin(y))<1e-3]
      print(os.path.basename(dump),'court',n,'model',i,'yaw %.4f lift %g'%(y,lift),'matrices at eye+lift:',len(k),'rotation ok:',len(ok))
