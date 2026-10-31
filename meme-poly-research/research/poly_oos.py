"""Out-of-sample copy test. Cutoff C = 2026-10-01 UTC.
Pool: every wallet with >=25 resolved buys before C and >=15 after C (from its last <=1500 trades).
Selection uses ONLY pre-C trades (their hold-to-resolution return at their price).
Test: delayed-copy returns on post-C trades. Reuses poly_copysim helpers and caches."""
import json,os,glob,time,sys,statistics as st,math,concurrent.futures as cf
import poly_copysim as pc
import argparse
ap=argparse.ArgumentParser(); ap.add_argument('--pre_end',default='2026-09-01'); ap.add_argument('--post_end',default='2026-10-01')
ap.add_argument('--out',default='oos_sep'); ap.add_argument('--wallets',default=None); A,_=ap.parse_known_args()
ts=lambda s: int(time.mktime(time.strptime(s,'%Y-%m-%d')))-time.timezone
C=ts(A.pre_end); E=ts(A.post_end); OUT=A.out
os.makedirs(OUT,exist_ok=True)
def sample(w,lo,hi,k):
    tr=json.load(open(f'wallets/{w}.json'))['trades']
    buys=sorted([t for t in tr if t['side']=='BUY' and 0.02<=t['price']<=0.98 and lo<=t['timestamp']<hi],key=lambda t:-t['timestamp'])
    return buys
def run(w):
    fn=f'{OUT}/{w}.json'
    if os.path.exists(fn): return json.load(open(fn))
    tr=json.load(open(f'wallets/{w}.json'))['trades']
    if not tr or min(t['timestamp'] for t in tr)>C-14*86400: json.dump(None,open(fn,'w')); return None
    pre=sample(w,0,C,0); post=sample(w,C,E,0)
    pc.resolve(list({t['conditionId'] for t in pre+post}))
    def pick(buys,k):
        seen=set(); out=[]
        for t in buys:
            g=pc.gamma.get(t['conditionId'])
            if not g or not g['closed'] or max(g['px'])<0.99: continue
            key=(t['asset'],t['timestamp']//3600)
            if key in seen: continue
            seen.add(key); out.append(t)
            if len(out)>=k: break
        return out
    a=pick(pre,40); b=pick(post,40)
    if len(a)<25 or len(b)<15: json.dump(None,open(fn,'w')); return None
    def one(t):
        g=pc.gamma[t['conditionId']]
        try: payoff=g['px'][g['toks'].index(t['asset'])]
        except ValueError: return None
        row=dict(ts=t['timestamp'],their_ret=payoff/t['price']-1,usd=t['size']*t['price'],px=t['price'])
        h=pc.path(t['asset'],t['timestamp'])
        for d in pc.DELAYS:
            p=pc.at(h,t['timestamp']+d) if h else None
            if p is None or p<=0 or p>=1: row[f'c{d}']=None; continue
            fill=min(0.999,p+pc.HALF_SPREAD); row[f'c{d}']=payoff/fill-1-g['fee']*(1-fill)
        return row
    pre_rows=[dict(ts=t['timestamp'],their_ret=(pc.gamma[t['conditionId']]['px'][pc.gamma[t['conditionId']]['toks'].index(t['asset'])])/t['price']-1) for t in a if t['asset'] in pc.gamma[t['conditionId']]['toks']]
    with cf.ThreadPoolExecutor(4) as ex: post_rows=[r for r in ex.map(one,b) if r]
    res=dict(wallet=w,pre=pre_rows,post=post_rows); json.dump(res,open(fn,'w')); return res
if __name__=='__main__':
    ws=json.load(open(A.wallets)) if A.wallets else [f.split('/')[-1][:-5] for f in glob.glob('wallets/*.json')]
    for i,w in enumerate(ws):
        r=run(w)
        if r: print(i,w[:10],'pre',len(r['pre']),'post',len(r['post']),flush=True)
    json.dump(pc.gamma,open(pc.GCACHE,'w'))
    print('DONE',flush=True)
