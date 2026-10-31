"""Skill statistics per Polymarket wallet from closed positions (realized) + trades (behaviour)."""
import json,glob,time,statistics as st,collections,re,math
NOW=time.time()
CAT=[('sports',r'nba|nfl|mlb|nhl|epl|ucl|fif|soccer|spread|-vs-|uefa|tennis|atp|wta|ufc|f1|cs2|lol|dota|valorant|cbb|cfb|ncaa|game|match'),
     ('politics',r'elect|senate|house|president|trump|governor|mayor|party|midterm|democrat|republican|vote|primary'),
     ('crypto',r'bitcoin|btc|ethereum|eth|solana|xrp|crypto|up-or-down'),
     ('econ',r'fed|rate|cpi|inflation|gdp|jobs|recession|tariff'),
     ('geo',r'iran|israel|ukraine|russia|china|war|ceasefire|strike')]
def cat(slug):
    for name,pat in CAT:
        if re.search(pat,slug or ''): return name
    return 'other'
def main():
    rows=[]
    
    if __name__=="__main__" or True:
      pass
    for f in glob.glob('wallets/*.json'):
        d=json.load(open(f)); w=f.split('/')[-1][:-5]; cp=d['closed']; tr=d['trades']
        if len(cp)<15: continue
        pos=[]
        for p in cp:
            cost=(p.get('avgPrice') or 0)*(p.get('totalBought') or 0)
            if cost<=0: continue
            pos.append(dict(t=p['timestamp'],pnl=p['realizedPnl'],cost=cost,ret=p['realizedPnl']/cost,avg=p['avgPrice'],cat=cat((p.get('eventSlug') or '')+' '+(p.get('slug') or ''))))
        if len(pos)<15: continue
        pos.sort(key=lambda x:x['t'])
        pnl=[x['pnl'] for x in pos]; tot=sum(pnl); srt=sorted(pnl,reverse=True)
        eq=0;peak=0;mdd=0
        for x in pnl: eq+=x; peak=max(peak,eq); mdd=min(mdd,eq-peak)
        win=[x for x in pos if x['pnl']>0]
        def window(days): return sum(x['pnl'] for x in pos if x['t']>NOW-days*86400), sum(1 for x in pos if x['t']>NOW-days*86400)
        cats=collections.Counter(x['cat'] for x in pos)
        buys=[t for t in tr if t['side']=='BUY']; sells=[t for t in tr if t['side']=='SELL']
        span_days=max(1,(max(t['timestamp'] for t in tr)-min(t['timestamp'] for t in tr))/86400) if len(tr)>1 else 1
        hi=[t for t in buys if t['price']>=0.90]
        # monthly consistency: share of positive months
        bym=collections.defaultdict(float)
        for x in pos: bym[time.strftime('%Y-%m',time.gmtime(x['t']))]+=x['pnl']
        rets=[x['ret'] for x in pos]
        rows.append(dict(wallet=w,name=d['meta']['name'],boards=d['meta']['boards'],n_closed=len(pos),capped=len(cp)>=1000,
            realized=tot,win_rate=len(win)/len(pos),avg_ret=st.mean(rets),med_ret=st.median(rets),
            roi_on_cost=tot/sum(x['cost'] for x in pos),top1_share=srt[0]/tot if tot>0 else None,
            ex_top1=tot-srt[0],ex_top5=tot-sum(srt[:5]),max_dd=mdd,dd_to_profit=abs(mdd)/tot if tot>0 else None,
            pnl7=window(7),pnl30=window(30),pnl90=window(90),months=len(bym),pos_months=sum(1 for v in bym.values() if v>0),
            cats=dict(cats.most_common(3)),trades=len(tr),trades_per_day=len(tr)/span_days,sell_share=len(sells)/max(1,len(tr)),
            hi_price_buy_share=len(hi)/max(1,len(buys)),avg_buy_usd=st.mean(t['size']*t['price'] for t in buys) if buys else 0,
            first=time.strftime('%Y-%m-%d',time.gmtime(pos[0]['t'])),last=time.strftime('%Y-%m-%d',time.gmtime(pos[-1]['t']))))
    # risk-adjusted score: per-position t-stat of returns, require breadth
    for r in rows:
        pass
    def tstat(w):
        d=json.load(open(f'wallets/{w}.json'))
        rets=[p['realizedPnl']/(p['avgPrice']*p['totalBought']) for p in d['closed'] if p.get('avgPrice') and p.get('totalBought')]
        return st.mean(rets)/(st.pstdev(rets)/math.sqrt(len(rets))) if len(rets)>2 and st.pstdev(rets)>0 else 0
    for r in rows: r['t_stat']=tstat(r['wallet'])
    json.dump(rows,open('poly_skill.json','w'),indent=1)
    print('wallets analysed',len(rows))
    q=[r for r in rows if r['realized']>0 and r['ex_top5']>0 and r['n_closed']>=40 and r['pos_months']>=max(2,0.6*r['months'])]
    print('pass filters (profitable ex-top5, >=40 closed, >=60% positive months):',len(q))
    q.sort(key=lambda r:-r['t_stat'])
    for r in q[:30]:
        print(f"{r['name'][:16]:16s} {r['wallet']} n={r['n_closed']:4d}{'+' if r['capped'] else ' '} real=${r['realized']/1e3:8.1f}k win={r['win_rate']*100:3.0f}% avgRet={r['avg_ret']*100:+6.1f}% t={r['t_stat']:+5.1f} top1={(r['top1_share'] or 0)*100:3.0f}% exTop5=${r['ex_top5']/1e3:7.1f}k mdd=${r['max_dd']/1e3:7.1f}k months {r['pos_months']}/{r['months']} 30d=${r['pnl30'][0]/1e3:6.1f}k tpd={r['trades_per_day']:5.1f} sell={r['sell_share']*100:3.0f}% hiPx={r['hi_price_buy_share']*100:3.0f}% avgBuy=${r['avg_buy_usd']:7.0f} {r['cats']}")
    
if __name__=="__main__": main()
