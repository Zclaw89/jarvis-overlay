"""Independent on-chain check of Kolscan wallets via public Solana RPC (publicnode).
For each wallet: last N transactions -> per-tx SOL delta and token deltas for the wallet -> buys/sells per mint.
Output: realized SOL PnL on fully-or-partly exited mints in the sample, trade counts, hold times, per-mint results."""
import requests,json,time,sys,os,collections,concurrent.futures as cf
RPC='https://solana-rpc.publicnode.com'; S=requests.Session()
WSOL='So11111111111111111111111111111111111111112'
STABLE={'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v','Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB'}
def rpc(method,params,tries=5):
    for i in range(tries):
        try:
            j=S.post(RPC,json={"jsonrpc":"2.0","id":1,"method":method,"params":params},timeout=40).json()
            if 'result' in j: return j['result']
            if j.get('error',{}).get('code') in (-32015,-32602,-32009,-32007): return None  # permanent
        except Exception: pass
        time.sleep(1.5*(i+1))
    return None
def parse(w,tx):
    if not tx or tx['meta'] is None or tx['meta'].get('err'): return None
    ak=tx['transaction']['message']['accountKeys']
    keys=[k['pubkey'] if isinstance(k,dict) else k for k in ak]
    if w not in keys: return None
    if not any(isinstance(k,dict) and k['pubkey']==w and k.get('signer') for k in ak): return None  # only txs the wallet signed
    i=keys.index(w); m=tx['meta']
    sol=(m['postBalances'][i]-m['preBalances'][i])/1e9
    tok=collections.defaultdict(float)
    for side,sign in (('preTokenBalances',-1),('postTokenBalances',1)):
        for b in m.get(side) or []:
            if b.get('owner')==w:
                tok[b['mint']]+=sign*float(b['uiTokenAmount']['uiAmount'] or 0)
    sol+=tok.pop(WSOL,0.0)
    tok={k:v for k,v in tok.items() if abs(v)>0 and k not in STABLE}
    return dict(t=tx['blockTime'],sol=sol,tok=tok,fee=m['fee']/1e9)
def wallet(w,n,max_pages=12):
    sigs=[]; ok=[]; before=None
    for _ in range(max_pages):
        r=rpc('getSignaturesForAddress',[w,{"limit":1000,**({"before":before} if before else {})}])
        if not r: break
        sigs+=r; ok+=[s for s in r if not s.get('err')]; before=r[-1]['signature']
        if len(ok)>=n or len(r)<1000: break
    ok=ok[:n]
    txs=list(map(lambda s: rpc('getTransaction',[s['signature'],{"encoding":"jsonParsed","maxSupportedTransactionVersion":1}]), ok))
    ev=[e for e in (parse(w,t) for t in txs) if e]
    span=(ok[0]['blockTime']-ok[-1]['blockTime'])/3600 if len(ok)>1 else 0
    return dict(wallet=w,n_sigs=len(sigs),n_ok=len(ok),n_events=len(ev),span_hours=span,events=sorted(ev,key=lambda e:e['t']))
if __name__=='__main__':
    board=json.load(open('kolscan_board.json'))
    targets=sys.argv[1:] or []
    os.makedirs('chain',exist_ok=True)
    for w in targets:
        fn=f'chain/{w}.json'
        if os.path.exists(fn): continue
        t=time.time(); d=wallet(w,1500); d['kolscan']=board.get(w)
        json.dump(d,open(fn,'w')); print(w,d['n_sigs'],'sigs',d['n_ok'],'ok',d['n_events'],'events','span %.1fh'%d['span_hours'],'%.0fs'%(time.time()-t),flush=True)
    print('DONE',flush=True)
