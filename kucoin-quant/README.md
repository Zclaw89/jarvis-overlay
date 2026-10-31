# kucoin-quant — $500 KuCoin spot research & paper-trading system

Read-only by default. **No live order can be sent** unless three independent switches are set
(`live.enabled: true` in `config.yaml`, `KQ_LIVE=1`, and `KQ_AUTHORIZED="I AUTHORIZE LIVE TRADING"`).

The full research write-up, evidence, allocation and rules are in [`PROPOSAL.md`](PROPOSAL.md).

## Layout
```
kq/kucoin.py     KuCoin public REST client (symbols, tickers, candles w/ paging, book impact, announcements) + gated private client
kq/intel.py      DexScreener / GeckoTerminal / CoinGecko / Polymarket / Kalshi / KuCoin-announcement collectors -> KuCoin overlap
kq/strategy.py   Primary strategy rules ("liquid capitulation reversion") — pure functions, no I/O
kq/scanner.py    Hourly scan -> data/signals.jsonl (+ optional intel snapshot -> data/intel_snapshots.jsonl)
kq/paper.py      Paper fills at next 1h open, stop/time exits, daily-loss & drawdown halts -> data/paper_trades.jsonl
kq/dashboard.py  Static HTML dashboard -> data/dashboard.html
research/        Scripts that produced every number in PROPOSAL.md, plus results/ JSON and the data snapshots used
```

## Run
```bash
pip install -r requirements.txt
python -m kq.scanner --intel      # ~2-3 min: pulls 1h candles for the ~120 most liquid eligible USDT pairs
python -m kq.paper step           # process signals / close due positions (schedule hourly at :02)
python -m kq.paper report
python -m kq.dashboard            # open data/dashboard.html
```
Cron (UTC): `2 * * * * cd /path/kucoin-quant && python -m kq.scanner --intel && python -m kq.paper step && python -m kq.dashboard`

## Reproduce the research
```bash
cd research
python fetch_1h.py          # ~125 days of 1h candles for liquid pairs (writes candles_1h.json, ~24 MB, git-ignored)
python backtest_1h.py       # strategies A-E, costs included
python robust_E.py          # robustness of the selected strategy (point-in-time liquidity, exclusions, 2x costs, walk-forward)
python listing_study.py     # new-listing event study, 93 listings
python poly_leadlag.py      # Polymarket BTC daily market vs KuCoin BTC lead/lag
```
