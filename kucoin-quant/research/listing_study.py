"""Event study: KuCoin spot new listings (last 180 days). Buys at first 5-min close after trading start,
measures forward return at horizons. Uses only KuCoin public candles."""
import json, time, requests, datetime as dt, statistics as st
S=json.load(open('data/symbols.json'))['data']
now=time.time(); cutoff=now-180*86400
ls=[(x['symbol'],x['tradingStartTime']/1000) for x in S if x.get('tradingStartTime') and x['quoteCurrency']=='USDT' and x['tradingStartTime']/1000>cutoff and x['enableTrading']]
ls.sort(key=lambda r:r[1]); print('listings in window:',len(ls))
sess=requests.Session(); out=[]
def get(url,**kw):
    for i in range(5):
        try: return sess.get(url,timeout=30,**kw).json()
        except Exception as e: time.sleep(2**i)
    return {}
H=[1,3,6,12,24,48,96]  # hours
for sym,t0 in ls:
    r=get('https://api.kucoin.com/api/v1/market/candles',params=dict(symbol=sym,type='5min',startAt=int(t0)-300,endAt=int(t0)+1400*300))
    time.sleep(0.15)
    if r.get('code')!='200000' or not r['data']: print('no data',sym); continue
    c=sorted(r['data'],key=lambda k:int(k[0]))  # [t,open,close,high,low,vol,turnover]
    first=c[0]; tf=int(first[0])
    if tf-t0>3600*6: print('late first candle',sym,(tf-t0)/3600); 
    entry=float(first[2])  # close of first 5-min candle
    o=float(first[1])
    row=dict(symbol=sym,start=dt.datetime.fromtimestamp(t0,dt.UTC).isoformat(),first_open=o,entry=entry,first_candle_ret=entry/o-1,
             turnover_1h=sum(float(k[6]) for k in c if int(k[0])<tf+3600),n=len(c))
    for h in H:
        tgt=tf+h*3600; k=[x for x in c if int(x[0])<=tgt]
        row[f'r{h}h']=float(k[-1][2])/entry-1 if k else None
    hi=max(float(k[3]) for k in c if int(k[0])<tf+24*3600); lo=min(float(k[4]) for k in c if int(k[0])<tf+24*3600)
    row['max_up_24h']=hi/entry-1; row['max_dd_24h']=lo/entry-1
    out.append(row); print(sym,row['start'][:10],f"1h {row['r1h']*100:+.1f}% 24h {row['r24h']*100:+.1f}% 4d {(row['r96h'] or 0)*100:+.1f}% turn1h ${row['turnover_1h']/1e3:.0f}k")
json.dump(out,open('listing_study.json','w'),indent=1)
print('\n=== summary (buy first 5m close, hold h) ===')
for h in H:
    v=[r[f'r{h}h'] for r in out if r.get(f'r{h}h') is not None]
    if v: print(f'{h:>4}h n={len(v)} mean={st.mean(v)*100:+.1f}% median={st.median(v)*100:+.1f}% win={sum(1 for x in v if x>0)/len(v)*100:.0f}% worst={min(v)*100:+.1f}% best={max(v)*100:+.1f}%')
