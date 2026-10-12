# B74a: the court load copies season 0's threshold/light/fog rows of envir_cNN.dat over seasons 1-3. For each RAM dump
# whose envir is loaded, prints the court manager's (court, hole, season, weather) and whether each season row in RAM
# equals the file's row 0 (copy) or its own row. Run from context/:  python3 ../research/envir_season_check.py ram/*.bin
import glob,struct,sys
for dump in sys.argv[1:]:
  raw=open(dump,'rb').read()[:32<<20]
  for p in sorted(glob.glob('p17k/xb/COURT/*/CMN.XB/data/court/plant/envir_c??.dat')):
    env=open(p,'rb').read(); i=raw.find(env[0:0x40])
    if i<0: continue
    ram=raw[i:i+len(env)]
    print(dump,'court',p.split('/')[3],'manager court,hole,season,weather',struct.unpack_from('<4i',raw,i-0x550+0x74))
    for name,b,st in [('threshold',0x10,0x30),('light',0xd0,0x70),('fog',0x290,0x60)]:
      for s in range(1,4):
        o=b+s*st
        print('  %-9s season %d: RAM = file row 0 %-5s RAM = file row %d %s'%(name,s,ram[o:o+st]==env[b:b+st],s,ram[o:o+st]==env[o:o+st]))
