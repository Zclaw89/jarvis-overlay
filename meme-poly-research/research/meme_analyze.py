"""From on-chain events: per-mint round trips, realized SOL PnL, hold time, sizes. Then delayed-copy simulation
using GeckoTerminal 1-minute OHLCV (USD).  Copy rule: buy at the 1m close at t_buy+d, sell at the 1m close at
t_their_first_sell+d (or hold to the end of available data, max 24h), costs = ROUND_TRIP (fees+slippage+priority)."""
import json,glob,time,os,sys,statistics as st,collections,requests
S=requests.Session(); GT='https://api.geckoterminal.com/api/v2'
ROUND_TRIP=0.03   # pump.fun/pumpswap ~1% per side incl. creator+protocol fees, + ~0.5% slippage per side at small size
os.makedirs('gt',exist_ok=True)
def gt(path,params=None):
    for i in range(5):
        try:
            r=S.get(GT+path,params=params,timeout=30,headers={'accept':'application/json'})
            if r.status_code==200: time.sleep(2.1); return r.json()
            if r.status_code==429: time.sleep(15*(i+1)); continue
            if r.status_code==404: return None
        except Exception: pass
        time.sleep(3)
def round_trips(ev):
    by=collections.defaultdict(list)
    for e in ev:
        for mint,amt in e['tok'].items():
            by[mint].append(dict(t=e['t'],amt=amt,sol=e['sol']))
    rts=[]
    for mint,xs in by.items():
        buys=[x for x in xs if x['amt']>0 and x['sol']<-0.005]; sells=[x for x in xs if x['amt']<0 and x['sol']>0.0]
        if not buys: continue
        bq=sum(x['amt'] for x in buys); bc=-sum(x['sol'] for x in buys)
        sq=-sum(x['amt'] for x in sells); sp=sum(x['sol'] for x in sells)
        frac=min(1.0,sq/bq) if bq>0 else 0
        rts.append(dict(mint=mint,t_buy=buys[0]['t'],t_sell=sells[0]['t'] if sells else None,t_last=max(x['t'] for x in xs),
                        cost_sol=bc,proceeds_sol=sp,sold_frac=frac,realized_sol=sp-bc*frac,ret=(sp/(bc*frac)-1) if frac>0.05 else None,
                        n_buys=len(buys),n_sells=len(sells)))
    return rts
def ohlcv(mint,t0,t1):
    fn=f'gt/{mint}_{t0//3600}.json'
    if os.path.exists(fn): return json.load(open(fn))
    pools=gt(f'/networks/solana/tokens/{mint}/pools',dict(page=1))
    if not pools or not pools.get('data'): json.dump([],open(fn,'w')); return []
    pool=pools['data'][0]['attributes']['address']
    d=gt(f'/networks/solana/pools/{pool}/ohlcv/minute',dict(aggregate=1,before_timestamp=t1,limit=1000,currency='usd',token='base'))
    rows=sorted(((int(c[0]),float(c[4])) for c in (d or {}).get('data',{}).get('attributes',{}).get('ohlcv_list',[])))
    json.dump(rows,open(fn,'w')); return rows
def px_at(rows,ts):
    for t,c in rows:
        if t>=ts-30: return c,t
    return None,None
def main(delays=(0,30,60,300)):
    out={}
    os.makedirs('meme_out',exist_ok=True)
    for f in sorted(glob.glob('chain/*.json')):
        d=json.load(open(f)); w=d['wallet']
        if os.path.exists(f'meme_out/{w}.json'): out[w]=json.load(open(f'meme_out/{w}.json')); continue
        rts=round_trips(d['events'])
        closed=[r for r in rts if r['ret'] is not None]
        real=sum(r['realized_sol'] for r in closed); rets=[r['ret'] for r in closed]
        srt=sorted((r['realized_sol'] for r in closed),reverse=True)
        holds=[(r['t_sell']-r['t_buy'])/60 for r in closed if r['t_sell']]
        kol=d.get('kolscan') or {}
        s=dict(wallet=w,name=(list(kol.values())[0]['name'] if kol else ''),kol={k:(round(v['profit']),v['wins'],v['losses']) for k,v in kol.items()},
               n_ok=d.get('n_ok'),span_h=round(d['span_hours'],1),n_mints=len(rts),n_closed=len(closed),realized_sol=round(real,2),
               ex_top1_sol=round(real-(srt[0] if srt else 0),2),win=round(sum(1 for x in rets if x>0)/len(rets),2) if rets else None,
               med_ret=round(st.median(rets),3) if rets else None,med_hold_min=round(st.median(holds),1) if holds else None,
               med_cost_sol=round(st.median(r['cost_sol'] for r in rts),2) if rts else None,copy={})
        # delayed copy on up to 25 most recent closed round trips
        sample=sorted(closed,key=lambda r:-r['t_buy'])[:15]; sims=collections.defaultdict(list)
        for r in sample:
            exit_t=r['t_sell'] or min(r['t_buy']+86400,int(time.time()))
            rows=ohlcv(r['mint'],r['t_buy'],exit_t+600)
            if not rows: continue
            for dl in delays:
                pe,te=px_at(rows,r['t_buy']+dl); px,tx=px_at(rows,exit_t+dl)
                if pe and px and te-(r['t_buy']+dl)<=120: sims[dl].append(px/pe-1-ROUND_TRIP)
        for dl,v in sims.items():
            s['copy'][dl]=dict(n=len(v),mean=round(st.mean(v),3),median=round(st.median(v),3),win=round(sum(1 for x in v if x>0)/len(v),2))
        out[w]=s; json.dump(s,open(f'meme_out/{w}.json','w')); print(json.dumps(s),flush=True)
    json.dump(out,open('meme_results.json','w'),indent=1)
if __name__=='__main__': main()
