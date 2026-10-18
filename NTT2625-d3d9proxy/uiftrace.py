import sys; sys.path.insert(0,'.')
import uif3
orig=uif3.elem
log=[]
def traced(r,typ):
    start=r.o
    e=orig(r,typ)
    log.append((typ,e['type'],e.get('id'),e.get('rect'),start,r.o))
    return e
uif3.elem=traced
try:
    uif3.load(sys.argv[1]); print('OK')
except Exception as ex:
    print('ERR',ex)
    for x in log[-4:]: print('  ',x)
    d=open(sys.argv[1],'rb').read(); end=log[-1][5]
    print('next bytes', d[end:end+100].hex(' '))
