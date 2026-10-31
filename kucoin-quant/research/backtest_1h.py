"""Backtests on KuCoin 1h candles (read-only data). Costs: taker fee per side from ticker (0.1/0.2/0.3%) + slippage model = half-spread + 0.05%.
Strategies tested:
  A. 24h top-gainer mean-reversion (long-only can't short; so test: buying 24h gainers = momentum continuation?)  -> ret of buying top-N 24h gainers, hold 24h
  B. Volume-breakout momentum: 1h volume > k * 24h mean AND close > 24h high -> buy, exit after H hours or stop
  C. Cross-sectional 24h momentum (buy top decile, hold 24h), rebalanced daily
  D. Overnight dip-buy on majors (control)
All entries at NEXT candle open (no look-ahead)."""
import json,math,statistics as st,itertools,random
C=json.load(open('candles_1h.json')); U={r[0]:r for r in json.load(open('data/universe_rows.json'))}
D={}
for s,cs in C.items():
    if len(cs)<2000: continue
    D[s]=[(int(c[0]),float(c[1]),float(c[2]),float(c[3]),float(c[4]),float(c[6])) for c in cs]  # t,o,c,h,l,turnover
print('symbols',len(D))
def cost(s): fee=U[s][4]; spread=U[s][2]; return 2*fee+spread+0.001  # round trip fee + full spread + 0.1% slippage total
def summarize(name,trades):
    if not trades: print(name,'no trades'); return
    r=[t['net'] for t in trades]; w=[x for x in r if x>0]; l=[x for x in r if x<=0]
    pf=sum(w)/abs(sum(l)) if l else float('inf')
    print(f"{name}: n={len(r)} mean={st.mean(r)*100:+.2f}% median={st.median(r)*100:+.2f}% win={len(w)/len(r)*100:.0f}% avgW={st.mean(w)*100 if w else 0:+.2f}% avgL={st.mean(l)*100 if l else 0:+.2f}% PF={pf:.2f} t={st.mean(r)/ (st.pstdev(r)/math.sqrt(len(r))) if len(r)>2 and st.pstdev(r)>0 else 0:+.2f}")
    return dict(name=name,n=len(r),mean=st.mean(r),median=st.median(r),win=len(w)/len(r),pf=pf)
res=[]
# --- B. volume breakout
for k,H,stop in [(3,6,0.03),(3,24,0.05),(5,12,0.04),(5,24,0.06),(8,24,0.06)]:
    trades=[]
    for s,cs in D.items():
        i=48
        while i<len(cs)-H-2:
            t,o,c,h,l,v=cs[i]
            vol24=st.mean(x[5] for x in cs[i-24:i]); hi24=max(x[3] for x in cs[i-24:i])
            if vol24>0 and v>k*vol24 and c>hi24 and v>20000:
                e=cs[i+1][1]; sl=e*(1-stop); exitp=None
                for j in range(i+1,i+1+H):
                    if cs[j][4]<=sl: exitp=sl; break
                exitp=exitp or cs[i+H][2]
                trades.append(dict(s=s,t=t,net=exitp/e-1-cost(s))); i+=H
            i+=1
    r=summarize(f'B vol-breakout k={k} H={H}h stop={stop}',trades); res.append(r)
# --- A/C: buy top-N 24h gainers at 00:00 UTC, hold 24h (momentum) and bottom-N (reversion)
for N,side in [(5,'top'),(5,'bottom'),(10,'top'),(10,'bottom')]:
    trades=[]
    ts=sorted(set(c[0] for cs in D.values() for c in cs if c[0]%86400==0))
    idx={s:{c[0]:i for i,c in enumerate(cs)} for s,cs in D.items()}
    for t in ts:
        rows=[]
        for s,cs in D.items():
            i=idx[s].get(t)
            if i is None or i<25 or i+25>=len(cs): continue
            r24=cs[i][2]/cs[i-24][2]-1; turn=sum(x[5] for x in cs[i-24:i])
            if turn>3e5: rows.append((r24,s,i))
        rows.sort(reverse=(side=='top')); pick=rows[:N]
        for r24,s,i in pick:
            e=cs=D[s][i+1][1]; x=D[s][i+25][2]
            trades.append(dict(s=s,t=t,net=x/e-1-cost(s)))
    res.append(summarize(f'C 24h {side}-{N} by 24h return, hold 24h',trades))
# --- D control: BTC buy&hold-equivalent per-day return
btc=D.get('BTC-USDT'); 
if btc:
    d=[btc[i+24][2]/btc[i][2]-1 for i in range(0,len(btc)-24,24)]
    print(f'control BTC daily: mean={st.mean(d)*100:+.2f}% total={(btc[-1][2]/btc[0][2]-1)*100:+.1f}% over {len(btc)/24:.0f} days')
# --- E: majors RSI-style dip buy: close < 24h low -> buy next open, hold 24h
trades=[]
for s,cs in D.items():
    if U[s][1]<5e6: continue
    for i in range(48,len(cs)-26):
        if cs[i][2]<min(x[4] for x in cs[i-48:i-1]) and cs[i][2]<cs[i-24][2]*0.95:
            e=cs[i+1][1]; trades.append(dict(s=s,t=cs[i][0],net=cs[i+25][2]/e-1-cost(s)))
res.append(summarize('E liquid-majors 48h-low dip buy (<-5% 24h), hold 24h',trades))
json.dump([r for r in res if r],open('backtest_1h.json','w'),indent=1)
