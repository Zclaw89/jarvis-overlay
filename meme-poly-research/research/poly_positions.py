"""Resumable: pull /positions (open + lost-but-unredeemed) for every pooled wallet."""
import requests,json,time,os
S=requests.Session(); B='https://data-api.polymarket.com'
pool=json.load(open('pool.json')); os.makedirs('positions',exist_ok=True)
def get(params):
    for i in range(4):
        try:
            r=S.get(B+'/positions',params=params,timeout=30)
            if r.status_code==200: return r.json()
        except Exception: pass
        time.sleep(1.5*(i+1))
for i,w in enumerate(pool):
    fn=f'positions/{w}.json'
    if os.path.exists(fn): continue
    out=[]
    for off in range(0,3000,500):
        d=get(dict(user=w,limit=500,offset=off,sizeThreshold=0))
        if not d: break
        out+=d
        if len(d)<500: break
    json.dump(out,open(fn,'w')); print(i,w[:10],len(out),flush=True)
print('DONE',flush=True)
