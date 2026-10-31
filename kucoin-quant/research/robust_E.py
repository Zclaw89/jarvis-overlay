import json,math,statistics as st,random,datetime as dt
C=json.load(open('candles_1h.json')); U={r[0]:r for r in json.load(open('data/universe_rows.json'))}
D={s:[(int(c[0]),float(c[1]),float(c[2]),float(c[3]),float(c[4]),float(c[6])) for c in cs] for s,cs in C.items() if len(cs)>=2000}
def cost(s): return 2*U[s][4]+U[s][2]+0.001
def summ(name,r):
    if len(r)<5: print(name,'n<5'); return
    w=[x for x in r if x>0]; l=[x for x in r if x<=0]; pf=sum(w)/abs(sum(l)) if l else 99
    se=st.pstdev(r)/math.sqrt(len(r)); print(f"{name:55s} n={len(r):4d} mean={st.mean(r)*100:+.2f}% med={st.median(r)*100:+.2f}% win={len(w)/len(r)*100:.0f}% PF={pf:.2f} t={st.mean(r)/se if se else 0:+.2f}")
def run(symfilter,lookback=48,drop=0.05,hold=24,dedupe=True,lowfield=4,stop=None):
    tr=[]
    for s,cs in D.items():
        if not symfilter(s): continue
        last=-999
        for i in range(lookback,len(cs)-hold-2):
            if i-last<hold and dedupe: continue
            if cs[i][2]<min(x[lowfield] for x in cs[i-lookback:i-1]) and cs[i][2]<cs[i-24][2]*(1-drop):
                e=cs[i+1][1]; x=cs[i+hold+1][2]
                if stop:
                    sl=e*(1-stop)
                    for j in range(i+1,i+hold+1):
                        if cs[j][4]<=sl: x=sl; break
                tr.append((cs[i][0],s,x/e-1-cost(s))); last=i
    return tr
liq=lambda s:U[s][1]>=5e6
base=run(liq); summ('E deduped (1 trade/symbol/24h)',[t[2] for t in base])
summ('E deduped, 3% stop',[t[2] for t in run(liq,stop=0.03)])
summ('E deduped, 8% stop',[t[2] for t in run(liq,stop=0.08)])
# monthly split
bym={}
for t,s,r in base: bym.setdefault(dt.datetime.fromtimestamp(t,dt.UTC).strftime('%Y-%m'),[]).append(r)
for m in sorted(bym): summ(f'  month {m}',bym[m])
# regime: BTC 7d trend at signal time
btc={c[0]:c[2] for c in D['BTC-USDT']}
up=[r for t,s,r in base if t in btc and (t-168*3600) in btc and btc[t]>btc[t-168*3600]]
dn=[r for t,s,r in base if t in btc and (t-168*3600) in btc and btc[t]<=btc[t-168*3600]]
summ('  BTC 7d uptrend at signal',up); summ('  BTC 7d downtrend at signal',dn)
# random-entry benchmark on same symbols, same hold, same cost
random.seed(1); rnd=[]
for s,cs in D.items():
    if not liq(s): continue
    for _ in range(40):
        i=random.randrange(48,len(cs)-26); rnd.append(cs[i+25][2]/cs[i+1][1]-1-cost(s))
summ('  random-entry benchmark (same symbols/costs)',rnd)
# ex-BTC/ETH, and non-majors
summ('E mid-liquidity 1M-5M',[t[2] for t in run(lambda s:1e6<=U[s][1]<5e6)])
summ('E low-liquidity 0.5M-1M',[t[2] for t in run(lambda s:5e5<=U[s][1]<1e6)])
# parameter grid
for lb,dp,h in [(24,0.05,24),(48,0.08,24),(48,0.05,12),(48,0.05,48),(72,0.10,24),(48,0.03,24)]:
    summ(f'E grid lb={lb} drop={dp} hold={h}',[t[2] for t in run(liq,lb,dp,h)])
# walk-forward: first half vs second half
mid=sorted(t[0] for t in base)[len(base)//2]
summ('  first half',[r for t,s,r in base if t<mid]); summ('  second half',[r for t,s,r in base if t>=mid])
# per-symbol concentration
bys={}
for t,s,r in base: bys.setdefault(s,[]).append(r)
top=sorted(bys.items(),key=lambda kv:-sum(kv[1]))[:8]; print('top contributors:',[(s,len(v),round(sum(v)*100,1)) for s,v in top])
print('symbols with positive mean:',sum(1 for v in bys.values() if st.mean(v)>0),'/',len(bys))
json.dump(dict(trades=base),open('E_trades.json','w'))
print('\n=== point-in-time liquidity + exclusions ===')
def run_pit(minturn=3e6,exclude=(),lookback=48,drop=0.05,hold=24,costmult=1.0,stop=None):
    tr=[]
    for s,cs in D.items():
        if s in exclude or s.startswith(('USDC','USDT')): continue
        last=-999
        for i in range(lookback,len(cs)-hold-2):
            if i-last<hold: continue
            turn24=sum(x[5] for x in cs[i-24:i])
            if turn24<minturn: continue
            if cs[i][2]<min(x[4] for x in cs[i-lookback:i-1]) and cs[i][2]<cs[i-24][2]*(1-drop):
                e=cs[i+1][1]; x=cs[i+hold+1][2]
                if stop:
                    sl=e*(1-stop)
                    for j in range(i+1,i+hold+1):
                        if cs[j][4]<=sl: x=sl; break
                tr.append((cs[i][0],s,x/e-1-cost(s)*costmult)); last=i
    return tr
for mt in [1e6,3e6,10e6]:
    summ(f'PIT turnover>={mt/1e6:.0f}M',[t[2] for t in run_pit(mt)])
summ('PIT 3M, excl JCT/Q/AIN',[t[2] for t in run_pit(3e6,('JCT-USDT','Q-USDT','AIN-USDT'))])
summ('PIT 3M, excl top8 contributors',[t[2] for t in run_pit(3e6,tuple(s for s,_ in top))])
summ('PIT 3M, costs x2',[t[2] for t in run_pit(3e6,costmult=2)])
summ('PIT 3M, 8% stop',[t[2] for t in run_pit(3e6,stop=0.08)])
summ('PIT 3M, 10% stop',[t[2] for t in run_pit(3e6,stop=0.10)])
summ('PIT 3M, drop 8%',[t[2] for t in run_pit(3e6,drop=0.08)])
summ('PIT 3M, drop 8%, 10% stop',[t[2] for t in run_pit(3e6,drop=0.08,stop=0.10)])
pit=run_pit(3e6); bys={}
for t,s,r in pit: bys.setdefault(s,[]).append(r)
print('PIT symbols',len(bys),'positive-mean symbols',sum(1 for v in bys.values() if st.mean(v)>0))
print('top contributors:',[(s,len(v),round(sum(v)*100,1)) for s,v in sorted(bys.items(),key=lambda kv:-sum(kv[1]))[:8]])
# daily-level equity sim: $500, 25% per trade max 2 concurrent -> approximate: sum of trades per day
byd={}
for t,s,r in pit: byd.setdefault(t//86400,[]).append(r)
eq=500.0; peak=500; mdd=0; days=sorted(byd)
for d in days:
    rs=byd[d][:2]
    for r in rs: eq+= 125*r
    peak=max(peak,eq); mdd=min(mdd,eq/peak-1)
print(f'equity sim ($125/trade, max 2/day): end=${eq:.0f} maxDD={mdd*100:.1f}% days-with-signal={len(days)} total trades={sum(min(2,len(v)) for v in byd.values())}')
json.dump(dict(trades=pit),open('E_trades_pit.json','w'))
