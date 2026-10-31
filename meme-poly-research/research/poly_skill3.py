"""Honest per-wallet record over a COMPLETE window.
closed_ts = closed positions newest-first (includes redeemed losers); positions = open + unredeemed losers (price 0) + unredeemed winners.
Window start = timestamp of the oldest closed row we have if the list hit the 1000 cap (else 90 days back), so nothing is missing inside it."""
import json,glob,os,time,statistics as st,math,collections
from poly_skill import cat
NOW=time.time(); pool=json.load(open('pool.json'))
def ts_end(p):
    try: return int(time.mktime(time.strptime(p['endDate'][:10],'%Y-%m-%d')))-time.timezone
    except Exception: return None
rows=[]
for w,meta in pool.items():
    fc,fp,fw=f'closed_ts/{w}.json',f'positions/{w}.json',f'wallets/{w}.json'
    if not (os.path.exists(fc) and os.path.exists(fp)): continue
    C=json.load(open(fc)); P=json.load(open(fp)); tr=json.load(open(fw))['trades'] if os.path.exists(fw) else []
    if len(C)<20: continue
    start=min(x['timestamp'] for x in C) if len(C)>=1000 else NOW-90*86400
    days=(NOW-start)/86400
    seen=set(); pos=[]
    for p in C:
        if p['timestamp']<start: continue
        cost=(p.get('avgPrice') or 0)*(p.get('totalBought') or 0)
        if cost<=1: continue
        seen.add(p['asset']); pos.append(dict(t=p['timestamp'],pnl=p['realizedPnl'],cost=cost,cat=cat((p.get('eventSlug') or '')+' '+(p.get('slug') or ''))))
    open_cost=open_val=0
    for p in P:
        if p['asset'] in seen: continue
        cost=p.get('initialValue') or 0
        if p.get('redeemable') and p['curPrice'] in (0,1):
            t=ts_end(p)
            if t and t>=start and cost>1: pos.append(dict(t=t,pnl=p['cashPnl'],cost=cost,cat=cat((p.get('eventSlug') or '')+' '+(p.get('slug') or ''))))
        elif 0<p['curPrice']<1: open_cost+=cost; open_val+=p.get('currentValue') or 0
    if len(pos)<30: continue
    pos.sort(key=lambda x:x['t']); pnl=[x['pnl'] for x in pos]; tot=sum(pnl); srt=sorted(pnl,reverse=True)
    eq=peak=mdd=0
    for x in pnl: eq+=x; peak=max(peak,eq); mdd=min(mdd,eq-peak)
    rets=[x['pnl']/x['cost'] for x in pos]
    t=st.mean(rets)/(st.pstdev(rets)/math.sqrt(len(rets))) if st.pstdev(rets)>0 else 0
    byw=collections.defaultdict(float)
    for x in pos: byw[int((x['t']-start)//(7*86400))]+=x['pnl']
    buys=[x for x in tr if x['side']=='BUY']; span=max(1,(max(x['timestamp'] for x in tr)-min(x['timestamp'] for x in tr))/86400) if len(tr)>1 else 1
    cost_tot=sum(x['cost'] for x in pos)
    rows.append(dict(wallet=w,name=meta['name'],window_days=round(days,1),n=len(pos),net=tot,win=sum(1 for x in pnl if x>0)/len(pnl),
        avg_ret=st.mean(rets),med_ret=st.median(rets),t=t,roi=tot/cost_tot,top1=srt[0]/tot if tot>0 else None,ex_top1=tot-srt[0],ex_top5=tot-sum(srt[:5]),
        mdd=mdd,mdd_pct_cost=mdd/(cost_tot/len(pos)) if pos else 0,weeks=len(byw),pos_weeks=sum(1 for v in byw.values() if v>0),
        pnl7=sum(x['pnl'] for x in pos if x['t']>NOW-7*86400),pnl30=sum(x['pnl'] for x in pos if x['t']>NOW-30*86400),
        open_cost=open_cost,open_unreal=open_val-open_cost,cats=dict(collections.Counter(x['cat'] for x in pos).most_common(2)),
        trades_per_day=len(tr)/span,sell_share=sum(1 for x in tr if x['side']=='SELL')/max(1,len(tr)),
        hi_px=sum(1 for x in buys if x['price']>=0.9)/max(1,len(buys)),med_buy=st.median(x['size']*x['price'] for x in buys) if buys else 0,
        avg_pos_cost=cost_tot/len(pos)))
json.dump(rows,open('poly_skill3.json','w'),indent=1)
print('wallets with complete-window records:',len(rows))
print('net negative over window:',sum(1 for r in rows if r['net']<=0), '| win-rate median %.2f'%st.median(r['win'] for r in rows))
def kind(r):
    if r['trades_per_day']>300 or (r['med_buy']<25 and r['trades_per_day']>50): return 'HFT bot'
    if r['sell_share']>0.4: return 'maker/trader'
    if r['hi_px']>0.5: return 'near-resolution'
    return 'directional'
for r in rows: r['kind']=kind(r)
print(collections.Counter(r['kind'] for r in rows))
q=[r for r in rows if r['net']>0 and r['ex_top5']>0 and r['n']>=40 and r['t']>=2 and r['pos_weeks']>=0.6*r['weeks']]
q.sort(key=lambda r:-r['t']); print('pass (net>0 ex-top5, n>=40, t>=2, >=60% positive weeks):',len(q))
for r in q[:30]:
    print(f"{r['kind'][:12]:12s} {r['name'][:16]:16s} {r['wallet']} win{r['window_days']:5.0f}d n={r['n']:4d} net=${r['net']/1e3:8.1f}k win={r['win']*100:3.0f}% medRet={r['med_ret']*100:+6.1f}% t={r['t']:+5.1f} roi={r['roi']*100:+5.1f}% top1={(r['top1'] or 0)*100:3.0f}% exTop5=${r['ex_top5']/1e3:7.1f}k mdd=${r['mdd']/1e3:7.1f}k wks {r['pos_weeks']}/{r['weeks']} 30d=${r['pnl30']/1e3:7.1f}k tpd={r['trades_per_day']:6.1f} medBuy=${r['med_buy']:6.0f} {r['cats']}")
json.dump(rows,open('poly_skill3.json','w'),indent=1)
