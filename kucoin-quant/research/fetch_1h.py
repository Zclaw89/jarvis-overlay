"""Download ~120 days of 1h candles for the most liquid KuCoin USDT pairs (read-only)."""
import json,time,requests
rows=json.load(open('data/universe_rows.json'))
syms=[r[0] for r in rows if r[1]>=5e5 and r[2]<=0.003 and not r[0].startswith(('USDC','USDT','DAI','TUSD','FDUSD','USDE','PYUSD','USDG','USD1'))][:160]
sess=requests.Session(); now=int(time.time()); out={}
for s in syms:
    cs=[]
    for i in range(2):
        end=now-i*1500*3600; start=end-1500*3600
        r=sess.get('https://api.kucoin.com/api/v1/market/candles',params=dict(symbol=s,type='1hour',startAt=start,endAt=end),timeout=30).json()
        time.sleep(0.12)
        if r.get('code')=='200000': cs+=r['data']
    cs=sorted({int(c[0]):c for c in cs}.values(),key=lambda c:int(c[0]))
    out[s]=cs
    print(s,len(cs),flush=True)
json.dump(out,open('candles_1h.json','w'))
print('done',len(out))
