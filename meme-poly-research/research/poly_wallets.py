"""Resumable: build pool of Polymarket wallets from v2 leaderboards; save closed positions + trades per wallet."""
import requests,json,time,os
S=requests.Session(); B='https://data-api.polymarket.com'
def get(path,params,tries=4):
    for i in range(tries):
        try:
            r=S.get(B+path,params=params,timeout=30)
            if r.status_code==200: return r.json()
        except Exception: pass
        time.sleep(1.5*(i+1))
    return None
if os.path.exists('pool.json'): pool=json.load(open('pool.json'))
else:
    pool={}
    for tp in ['week','month','all']:
        for cat in ['overall','sports','politics','crypto']:
            d=get('/v2/leaderboard',dict(limit=50,timePeriod=tp,category=cat))
            for x in (d or {}).get('data',[]):
                pool.setdefault(x['user_id'],dict(name=x['user_name'],boards=[]))['boards'].append((tp,cat,x['rank'],round(x['pnl']),round(x['volume'])))
            time.sleep(0.3)
    json.dump(pool,open('pool.json','w'))
print('pool size',len(pool),flush=True)
for i,(w,meta) in enumerate(pool.items()):
    fn=f'wallets/{w}.json'
    if os.path.exists(fn): continue
    cp=[]
    for off in range(0,1000,50):
        d=get('/closed-positions',dict(user=w,limit=50,offset=off))
        if not d: break
        cp+=d
        if len(d)<50: break
    tr=[]
    for off in range(0,1500,500):
        d=get('/trades',dict(user=w,limit=500,offset=off))
        if not d: break
        tr+=d
        if len(d)<500: break
    json.dump(dict(meta=meta,closed=cp,trades=tr),open(fn,'w'))
    print(i,w[:10],meta['name'][:14],'closed',len(cp),'trades',len(tr),flush=True)
print('DONE',flush=True)
