import json,glob,statistics as st,math,random,collections,sys
OUT=sys.argv[1] if len(sys.argv)>1 else 'oos'
pool=json.load(open('pool.json'))
R=[r for r in (json.load(open(f)) for f in glob.glob(OUT+'/*.json')) if r]
D=['c0','c60','c300','c900','c3600']
def tstat(v): return st.mean(v)/(st.pstdev(v)/math.sqrt(len(v))) if len(v)>2 and st.pstdev(v)>0 else 0
rows=[]
for r in R:
    pre=[x['their_ret'] for x in r['pre']]; post=r['post']
    if len(pre)<25 or len(post)<15: continue
    row=dict(w=r['wallet'],name=pool.get(r['wallet'],{}).get('name',''),n_pre=len(pre),pre_mean=st.mean(pre),pre_t=tstat(pre),
             n_post=len(post),post_their=st.mean(x['their_ret'] for x in post),
             med_px=st.median(x['px'] for x in post),med_usd=st.median(x['usd'] for x in post))
    for d in D:
        v=[x[d] for x in post if x.get(d) is not None]; row[d]=st.mean(v) if v else None; row[d+'_v']=v
    rows.append(row)
print('wallets in OOS test:',len(rows),' trades post:',sum(r['n_post'] for r in rows))
def group(name,g):
    out=[name,len(g)]
    for d in ['post_their']+D:
        if d=='post_their': v=[x['their_ret'] for r in g for x in R_by[r['w']]['post']]
        else: v=[a for r in g for a in r[d+'_v']]
        out.append('%+.3f (t%+.1f, n%d)'%(st.mean(v),tstat(v),len(v)) if v else '-')
    print(' | '.join(map(str,out)))
R_by={r['wallet']:r for r in R}
print('group | wallets | their post ret | copy@0 | copy@60s | copy@5m | copy@15m | copy@1h')
group('ALL wallets',rows)
srt=sorted(rows,key=lambda r:-r['pre_t'])
for k in (10,20,40): group(f'top{k} by PRE t-stat',srt[:k])
group('bottom half by PRE t-stat',srt[len(srt)//2:])
# pre-skill persistence: correlation of pre_mean with post copy@5m
xs=[r['pre_t'] for r in rows if r['c300'] is not None]; ys=[r['c300'] for r in rows if r['c300'] is not None]
mx,my=st.mean(xs),st.mean(ys); cov=sum((a-mx)*(b-my) for a,b in zip(xs,ys)); print('corr(pre t-stat, post copy@5m) = %.3f over %d wallets'%(cov/math.sqrt(sum((a-mx)**2 for a in xs)*sum((b-my)**2 for b in ys)),len(xs)))
# bootstrap: random 10-wallet portfolios
random.seed(7); sims=[]
for _ in range(2000):
    g=random.sample(rows,min(10,len(rows))); v=[a for r in g for a in r['c300_v']]; sims.append(st.mean(v))
sims.sort(); top10=[a for r in srt[:10] for a in r['c300_v']]
print('random 10-wallet copy@5m: median %+.3f, 5-95%% [%+.3f, %+.3f]; top10-by-pre: %+.3f (percentile %.0f)'%(sims[1000],sims[100],sims[1900],st.mean(top10),100*sum(1 for s in sims if s<st.mean(top10))/len(sims)))
# price buckets
allpost=[x for r in rows for x in R_by[r['w']]['post']]
for lo,hi in [(0,0.2),(0.2,0.5),(0.5,0.8),(0.8,1.0)]:
    v=[x['c300'] for x in allpost if lo<=x['px']<hi and x.get('c300') is not None]
    if v: print(f'entry price {lo:.1f}-{hi:.1f}: copy@5m mean {st.mean(v):+.3f} median {st.median(v):+.3f} win {sum(1 for a in v if a>0)/len(v):.2f} n={len(v)}')
print('\nTOP 20 by PRE t-stat, with OOS results')
for r in srt[:20]:
    print(f"{r['name'][:18]:18s} {r['w']} pre n={r['n_pre']} mean={r['pre_mean']:+.2f} t={r['pre_t']:+.1f} | post n={r['n_post']} their={r['post_their']:+.2f} copy5m={r['c300'] if r['c300'] is None else round(r['c300'],3)} copy1h={r['c3600'] if r['c3600'] is None else round(r['c3600'],3)} medPx={r['med_px']:.2f} medUSD={r['med_usd']:.0f}")
json.dump([{k:v for k,v in r.items() if not k.endswith('_v')} for r in srt],open(OUT+'_summary.json','w'),indent=1)
