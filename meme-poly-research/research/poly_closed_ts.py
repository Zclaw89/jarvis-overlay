"""Resumable: closed positions in TIMESTAMP DESC order (unbiased recent history, up to 1000) + trades if missing."""
import requests,json,time,os
S=requests.Session(); B='https://data-api.polymarket.com'
pool=json.load(open('pool.json')); os.makedirs('closed_ts',exist_ok=True)
def get(path,params):
    for i in range(4):
        try:
            r=S.get(B+path,params=params,timeout=30)
            if r.status_code==200: return r.json()
        except Exception: pass
        time.sleep(1.5*(i+1))
for i,(w,meta) in enumerate(pool.items()):
    fn=f'closed_ts/{w}.json'
    if os.path.exists(fn): continue
    cp=[]
    for off in range(0,1000,50):
        d=get('/closed-positions',dict(user=w,limit=50,offset=off,sortBy='TIMESTAMP',sortDirection='DESC'))
        if not d: break
        cp+=d
        if len(d)<50: break
    wf=f'wallets/{w}.json'
    if os.path.exists(wf): tr=json.load(open(wf))['trades']
    else:
        tr=[]
        for off in range(0,1500,500):
            d=get('/trades',dict(user=w,limit=500,offset=off))
            if not d: break
            tr+=d
            if len(d)<500: break
        json.dump(dict(meta=meta,closed=[],trades=tr),open(wf,'w'))
    json.dump(cp,open(fn,'w')); print(i,w[:10],meta['name'][:14],'closed_ts',len(cp),'trades',len(tr),flush=True)
print('DONE',flush=True)
