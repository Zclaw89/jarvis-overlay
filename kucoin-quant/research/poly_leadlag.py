import json,requests,time,math
s=json.load(open('data/poly_search.json'))
found={}
for e in s.get('events',[]):
    for m in e.get('markets',[]):
        q=m.get('question','')
        if q.startswith('Bitcoin Up or Down on October') and m.get('clobTokenIds') and '2026' in m.get('slug',''):
            found[m['slug']]=json.loads(m['clobTokenIds'])[0]
def corr(a,b):
    n=len(a); ma,mb=sum(a)/n,sum(b)/n
    cov=sum((x-ma)*(y-mb) for x,y in zip(a,b)); va=sum((x-ma)**2 for x in a); vb=sum((y-mb)**2 for y in b)
    return cov/math.sqrt(va*vb) if va and vb else float('nan')
res=[]
for q,tok in list(found.items())[:12]:
    h=requests.get('https://clob.polymarket.com/prices-history',params=dict(market=tok,interval='all',fidelity=1),timeout=30).json().get('history',[])
    time.sleep(1)
    if len(h)<100: print(q,'few points',len(h),tok[:12]); continue
    t0,t1=h[0]['t'],h[-1]['t']
    c=[]
    for a in range(t0-60,t1+60,1500*60):
        r=requests.get('https://api.kucoin.com/api/v1/market/candles',params=dict(symbol='BTC-USDT',type='1min',startAt=a,endAt=min(a+1500*60,t1+60)),timeout=30).json()
        c+=r.get('data',[]); time.sleep(0.1)
    px={}
    for k in c: px[(int(k[0])//600)*600]=float(k[2])
    pm={}
    for p in h: pm[(p['t']//600)*600]=p['p']
    ts=sorted(set(px)&set(pm))
    dp=[pm[ts[i+1]]-pm[ts[i]] for i in range(len(ts)-1)]; dx=[math.log(px[ts[i+1]]/px[ts[i]]) for i in range(len(ts)-1)]
    row={'q':q,'n':len(ts)}
    for lag in [-3,-2,-1,0,1,2,3]:
        if lag>=0: a=dp[:len(dp)-lag]; b=dx[lag:]
        else: a=dp[-lag:]; b=dx[:len(dx)+lag]
        row[lag]=round(corr(a,b),3)
    res.append(row); print(row)
json.dump(res,open('poly_leadlag.json','w'),indent=1)
