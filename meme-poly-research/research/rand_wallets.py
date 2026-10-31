"""Benchmark pool: wallets seen in the global trade feed (not selected on performance)."""
import requests,json,time,os,random
S=requests.Session(); B='https://data-api.polymarket.com'
seen={}
for off in range(0,1500,500):
    for _ in range(3):
        try:
            d=S.get(B+'/trades',params=dict(limit=500,offset=off),timeout=30).json(); break
        except Exception: time.sleep(2)
    for t in d: seen.setdefault(t['proxyWallet'],t.get('name',''))
pool=json.load(open('pool.json'))
cand=[w for w in seen if w not in pool]; random.seed(3); random.shuffle(cand); cand=cand[:120]
os.makedirs('wallets',exist_ok=True); done=[]
for w in cand:
    fn=f'wallets/{w}.json'
    if not os.path.exists(fn):
        tr=[]
        for off in range(0,1500,500):
            try: d=S.get(B+'/trades',params=dict(user=w,limit=500,offset=off),timeout=30).json()
            except Exception: break
            tr+=d
            if len(d)<500: break
        json.dump(dict(meta=dict(name=seen[w],boards=[]),closed=[],trades=tr),open(fn,'w'))
    done.append(w)
json.dump(done,open('rand_wallets.json','w')); print('random wallets',len(done),flush=True)
