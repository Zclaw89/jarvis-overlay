"""Delayed copy-trading simulation for Polymarket wallets.
For each sampled BUY by a wallet on a market that has since RESOLVED:
  their return    = payoff / their_price - 1                      (hold to resolution)
  copy return(d)  = payoff / (p(t+d) + half_spread) - 1 - fee(p)/p (we buy d seconds after their fill, hold to resolution)
p(t+d) comes from CLOB /prices-history at 1-minute fidelity (first point at or after t+d).
fee per share = feeSchedule.rate * p * (1-p)  (Polymarket formula; rate taken per market from Gamma).
Delays: 60s (API bot), 300s (alert + human), 900s, 3600s.  Also delay 0 using history price (sanity check vs their fill)."""
import requests,json,time,os,sys,collections,concurrent.futures as cf
S=requests.Session()
DELAYS=[0,60,300,900,3600]; HALF_SPREAD=0.01
os.makedirs('copysim',exist_ok=True)
GCACHE='copysim/gamma_cache.json'; gamma=json.load(open(GCACHE)) if os.path.exists(GCACHE) else {}
def get(url,params,tries=4):
    for i in range(tries):
        try:
            r=S.get(url,params=params,timeout=30)
            if r.status_code==200: return r.json()
        except Exception: pass
        time.sleep(1.5*(i+1))
def resolve(cids):
    need=[c for c in cids if c not in gamma]
    for i in range(0,len(need),20):
        chunk=need[i:i+20]
        for closed in ('true','false'):
            r=get('https://gamma-api.polymarket.com/markets',[('condition_ids',c) for c in chunk]+[('closed',closed),('limit',50)]) or []
            for m in r:
                try: toks=json.loads(m['clobTokenIds']); px=[float(x) for x in json.loads(m['outcomePrices'])]
                except Exception: continue
                gamma[m['conditionId']]=dict(closed=m.get('closed'),toks=toks,px=px,fee=(m.get('feeSchedule') or {}).get('rate',0) or 0,
                                             end=m.get('closedTime'),slug=m.get('slug'),min_size=m.get('orderMinSize'))
        for c in chunk: gamma.setdefault(c,None)
    json.dump(gamma,open(GCACHE,'w'))
def path(asset,t):
    h=get('https://clob.polymarket.com/prices-history',dict(market=asset,startTs=t-90,endTs=t+3700,fidelity=1))
    return (h or {}).get('history',[])
def at(hist,ts):
    for x in hist:
        if x['t']>=ts: return x['p']
    return None
def sim_wallet(w,max_trades=80):
    fn=f'copysim/{w}.json'
    if os.path.exists(fn): return json.load(open(fn))
    tr=json.load(open(f'wallets/{w}.json'))['trades']
    buys=sorted([t for t in tr if t['side']=='BUY' and 0.02<=t['price']<=0.98],key=lambda t:-t['timestamp'])
    resolve(list({t['conditionId'] for t in buys}))
    seen=set(); sample=[]
    for t in buys:
        g=gamma.get(t['conditionId'])
        if not g or not g['closed'] or max(g['px'])<0.99: continue  # resolved markets only
        key=(t['asset'],t['timestamp']//3600)
        if key in seen: continue
        seen.add(key); sample.append(t)
        if len(sample)>=max_trades: break
    out=[]
    def one(t):
        g=gamma[t['conditionId']]
        try: payoff=g['px'][g['toks'].index(t['asset'])]
        except ValueError: return None
        h=path(t['asset'],t['timestamp'])
        if not h: return None
        row=dict(ts=t['timestamp'],slug=t['slug'],their_px=t['price'],usd=t['size']*t['price'],payoff=payoff,fee_rate=g['fee'],
                 their_ret=payoff/t['price']-1)
        for d in DELAYS:
            p=at(h,t['timestamp']+d)
            if p is None or p<=0 or p>=1: row[f'c{d}']=None; continue
            fill=min(0.999,p+HALF_SPREAD); fee=g['fee']*fill*(1-fill)
            row[f'p{d}']=p; row[f'c{d}']=payoff/fill-1-fee/fill
        return row
    with cf.ThreadPoolExecutor(4) as ex:
        out=[r for r in ex.map(one,sample) if r]
    json.dump(out,open(fn,'w')); return out
if __name__=='__main__':
    ws=json.load(open(sys.argv[1]))
    for i,w in enumerate(ws):
        t0=time.time(); r=sim_wallet(w)
        print(i,w[:10],'n',len(r),'%.0fs'%(time.time()-t0),flush=True)
    print('DONE',flush=True)
