# $500 KuCoin Trading Proposal — research, evidence, and plan

*Prepared 2026-10-31 (UTC). Every number below was produced by the scripts in `research/` against live KuCoin and
third-party endpoints on that date. Nothing was traded. Live execution stays disabled until explicitly authorized.*

---

## A. Executive recommendation

**Primary strategy: "liquid capitulation reversion" on KuCoin spot, 1-hour timeframe.**
Buy a liquid USDT pair when its 1h close prints a new 48-hour low *and* is at least 5 % below its close 24 hours earlier;
exit 24 hours later or at a 10 % hard stop. Trade only pairs with ≥ $3 M trailing-24h KuCoin turnover, ≤ 15 bps spread,
no "st" (delisting-risk) flag, not listed in the last 14 days.

Why this and not the alternatives:

| Approach researched | Verdict | Evidence (see section H) |
|---|---|---|
| Capitulation reversion on liquid pairs | **Adopt (paper first)** | n=322 trades, mean +1.63 %, median +1.33 %, 67 % win, PF 2.49, t = 5.2 **after** fees, spread and slippage; survives 2× costs, exclusions, walk-forward, point-in-time liquidity |
| Volume-breakout momentum | Reject for now | positive only at extreme thresholds (k=8× volume): mean +2.1 %, 38 % win, PF 1.76, t = 2.0, n=306 — fragile |
| Buying 24h top gainers (trend-chasing) | Reject | mean −0.96 %, PF 0.79 |
| Buying 24h bottom losers (cross-sectional) | Reject | mean +0.13…+0.27 %, PF ~1.05, not significant |
| New-listing momentum | Reject | 93 listings: median return negative at every horizon (−0.2 % @1h … −5.7 % @4d); mean positive only via 3–4 lottery outcomes; worst −94 % |
| Polymarket BTC markets as a leading signal | Reject | Polymarket lags KuCoin BTC by ~1 minute (corr +0.52 at BTC→Poly lag 1, −0.004 the other way) |
| Meme/DEX trending → KuCoin | Not yet evaluable | no free historical feed; forward logger built (section F) — evaluate after 30 days of snapshots |
| Smart-money wallet copying | Not adopted | verifiable performance data sits behind paid APIs (Solscan Pro ≥ $199/mo, Birdeye ≥ $299/mo) — unjustifiable on $500 |

**Secondary strategy: none deployed.** Candidate for later evaluation: extreme volume breakout (k ≥ 8) with 24h hold,
only if the forward paper log confirms the backtest.

---

## B. Intelligence sources (verified 2026-10-31)

| Source | Endpoint tested | Access / cost | Use in system |
|---|---|---|---|
| KuCoin REST | `api.kucoin.com` `/api/v2/symbols`, `/api/v1/market/allTickers`, `/api/v1/market/candles`, `/api/v1/market/orderbook/level2_100`, `/api/v3/announcements` | free, no key for market data. Rate limit (docs): public pool 2000 weight/30 s per IP; private spot pool 4000/30 s at VIP0; HTTP 429 / code 429000 on breach ([rate-limit docs](https://www.kucoin.com/docs-new/rate-limit), [basic info](https://www.kucoin.com/docs/basic-info/request-rate-limit/rest-api)) | execution venue, candles, liquidity, fees |
| KuCoin WebSocket | `/api/v1/bullet-public` token → public feed | free | optional later; hourly polling suffices for a 1h strategy |
| DexScreener | `/token-boosts/top/v1`, `/token-profiles/latest/v1`, `/tokens/v1/{chain}/{addrs}` | free, documented 60 req/min on profile endpoints ([docs](https://docs.dexscreener.com/api/reference)) | forward-logged meme/DEX attention |
| GeckoTerminal | `/api/v2/networks/{solana,base,bsc}/trending_pools` | free, ~30 req/min (community figure; official docs do not state it) | forward-logged |
| CoinGecko | `/api/v3/search/trending` | free demo plan; returned HTTP 429 on 2 of 3 calls here — unreliable without a key ([rate-limit docs](https://docs.coingecko.com/docs/errors-and-rate-limits)) | forward-logged, best-effort |
| Polymarket | `gamma-api.polymarket.com/markets`, `clob.polymarket.com/prices-history` | free | tested and rejected as a leading signal; kept as context |
| Kalshi | `api.elections.kalshi.com/trade-api/v2/markets?series_ticker=KXBTC` | free read | context only (BTC range contracts had no quotes at test time) |
| Pump.fun frontend API | `frontend-api-v3.pump.fun` | HTTP 403 from this environment | not usable |
| Binance / Bybit | public APIs | HTTP 451 / 403 (geo-blocked from this environment) | cross-exchange lead/lag **not possible** from here; confirm from your own location |
| Solscan Pro / Birdeye | — | paid: Solscan Level 2 $199/mo ([levels](https://docs.solscan.io/api-access/solscan-pro-api-levels-and-details)); Birdeye data plans $299–449/mo per third-party pricing trackers | not used |

KuCoin fees, verified from the live ticker fields `takerFeeRate × takerCoefficient`: three classes exist on the 817 USDT pairs —
0.10 % (358 pairs), 0.20 % (215), 0.30 % (244). KuCoin's own blog states 20 % off when paying fees in KCS
([KuCoin fee breakdown](https://www.kucoin.com/blog/kucoin-fees-a-full-breakdown-before-trading-crypto)); the system assumes the undiscounted rate.
Minimum order: `minFunds` = 0.1 USDT on every USDT pair checked. Order types: limit (post-only, IOC, FOK), market, stop; the strategy uses
post-only limits. **Jurisdiction:** market data needs no account; spot trading availability for your country and the KYC tier must be
confirmed in the KuCoin app before day 1 (not verifiable from here).

---

## C. Market opportunities (live scan, 2026-10-31 13:57 UTC)

Universe after filters (USDT quote, trading enabled, not stable/fiat, not `st`, spread ≤ 15 bps, ≥ $3 M 24h turnover): **~95 pairs**.
Of 817 USDT pairs, 254 have > $100 k daily turnover, 81 > $1 M, 32 > $5 M. Anything below ~$1 M/day is unsuitable for a $125 order
(e.g. SONIC-USDT: $990 of asks within 1 % of mid; AIN-USDT: $550).

Live candidate produced by the scanner (`data/signals.jsonl`, quote timestamp 1793454321 = 13:45:21 UTC):

| Metric | TRX-USDT |
|---|---|
| Current price | 0.3207 at 1h candle close 13:00 UTC; book mid 0.31945 at 13:54 UTC |
| Opportunity | new 48h low and −6.5 % vs 24h-earlier close on a top-25 liquidity pair (capitulation setup) |
| Signals | KuCoin candles only (strategy condition 1–4 all true); no external source gates this trade |
| Liquidity | spread 3.1 bps, $125 market-order impact 2.5–4.7 bps, $27 k asks within 1 % of mid, 24h turnover $6.3 M |
| Entry | post-only limit at best bid during the first 3 minutes after the 14:00 UTC candle open; skip if not filled |
| Stop-loss | 0.28863 (−10 % from signal close); hard stop, taker exit |
| Profit target | time exit at the close 24 h after entry (backtest median +1.3 %, mean +1.6 %); no fixed price target |
| Position size | $125 (25 % of capital) |
| Risk | max loss ≈ $12.5 + ~$0.35 fees/slippage ≈ **$12.9** (2.6 % of capital) |
| Confidence | medium: setup matches the backtested class with 322 samples; single-trade win probability ≈ 0.67 historically; TRX is a top-10 asset so the −6.5 % move may be idiosyncratic (regulatory/news) — paper trade only |

The intel snapshot taken at the same time found these trending-token overlaps with KuCoin pairs: PENGU, PUMP, TROLL, LSK, STX, HYPE, LIT, PYTH
(GeckoTerminal Solana trending + CoinGecko trending). LSK was +47 % on the day with a 26 bps spread and $4.7 k 1 %-depth — **rejected** by liquidity.
These overlaps are being logged, not traded.

---

## D. Capital allocation (never exceeds $500)

| Bucket | Amount | Rule |
|---|---|---|
| Primary strategy working capital | $250 | max 2 concurrent positions × $125 |
| USDT reserve | $250 | untouched for the first 30 days; only reallocated after a passed review (section G) |
| Max position size | $125 | 25 % of capital |
| Risk per trade | $12.50 (2.5 %) | 10 % stop on $125 |
| Daily loss limit | $25 (5 %) | scanner refuses new entries for the UTC day |
| Max cumulative drawdown | $75 (15 %) from peak | system halts; manual review required to re-enable |
| API / infrastructure | $0 / month | all sources used are free; runs on any always-on machine (a $5/mo VPS optional) |
| Expected trading costs | ≈ 0.20–0.40 % fee round trip + 0.03–0.15 % spread + ~0.10 % slippage ≈ **0.35–0.65 % per trade** ($0.45–0.80 on $125) | modelled per symbol from live ticker fee class and spread |

Expected 30-day activity at this sizing: the backtest produced signals on 65 of 125 days and ~105 executable trades (≤ 2/day), i.e. about
**25 trades a month**, ~$15–20 of fees.

---

## E. Trading rules (exact)

**Universe (recomputed hourly):** USDT quote; `enableTrading`; base not in stable/fiat list; `st == false`; `tradingStartTime` older than 14 days;
top-of-book spread ≤ 15 bps; trailing-24h KuCoin turnover ≥ $3 M (computed from the candles themselves, point-in-time).

**Entry trigger** (evaluated at each 1h candle close `i`):
1. `close[i] < min(low[i-48 … i-2])`
2. `close[i] < close[i-24] × 0.95`
3. One signal per symbol per 24 h; max 2 open positions; no entry if daily loss limit or drawdown halt is active.
4. Book check at signal time: $125 impact ≤ 20 bps and ≥ $625 of asks within 1 % of mid, else reject.

**Order:** post-only limit at best bid placed at candle `i+1` open, re-pegged every 30 s for 3 minutes; unfilled → skip (no chasing).
Size = min($125, 25 % of current equity), rounded down to `baseIncrement`.

**Exits:** (a) hard stop 10 % below fill — monitored every minute, executed as taker limit-IOC at bid; (b) time exit: sell at the close of the
24th full hour after entry. No profit target, no trailing stop (tested: a 3 % stop destroys the edge, PF 1.18; 8–10 % stops keep PF ≈ 1.9–2.0).

**Invalidation of the strategy itself** (stop trading, back to paper): 30-trade rolling profit factor < 1.0, or win rate < 50 % over 40 trades,
or drawdown halt triggered twice in 30 days.

---

## F. Automation architecture

Python 3.11+, `requests`, `pyyaml`; polling, no WebSocket needed for a 1h strategy (WebSocket only for the 1-minute stop monitor in live mode).

```
cron @ :02 every hour
  kq.scanner  ── KuCoin /symbols + /allTickers ──► universe filter (liquidity, spread, st, age)
              ── KuCoin /candles (last 80 h, per symbol) ──► kq.strategy.evaluate (pure rules)
              ── KuCoin /orderbook level2_100 ──► impact check
              ── intel.snapshot(): DexScreener, GeckoTerminal, CoinGecko, Polymarket, Kalshi, KuCoin announcements
                 ──► cross-referenced to KuCoin pairs, appended to data/intel_snapshots.jsonl (timestamped)
              ──► data/signals.jsonl  (timestamp, price, book, size, stop, status)
  kq.paper    ── fills at next 1h open (+spread/2 +5 bps slippage +fee), stop/time exits, daily-loss + drawdown halts
              ──► data/paper_trades.jsonl, data/state.json
  kq.dashboard ──► data/dashboard.html  (equity, open positions, signals, intel overlap)
live mode (later): kq.kucoin.PrivateClient.place_limit_order — refuses unless config + KQ_LIVE=1 + KQ_AUTHORIZED phrase
```

Signal scoring from external sources is deliberately **not** in the trade path yet. Each snapshot stores (source, symbol, timestamp); after 30 days
the forward return of every "trending on KuCoin" mention is measured at 1/6/24 h vs. the universe average. A source earns a weight only if that
excess return is positive and significant net of costs; otherwise it stays informational. This avoids assigning confidence scores with no measured
predictive value.

---

## G. Implementation roadmap

**Days 1–7**
1. **Day 1** — Confirm KuCoin spot availability and KYC for your jurisdiction; create an API key with *General* permission only (no Trade, no
   Withdraw), IP-restricted; fund nothing yet. Enable "Pay fees with KCS" if you hold any KCS.
2. **Day 1** — Deploy `kucoin-quant` on an always-on machine; run `python -m kq.scanner --intel` manually; verify `data/signals.jsonl`.
3. **Day 2** — Hourly cron for scanner + paper + dashboard. Confirm it survives KuCoin 429s and proxy resets (retry/backoff is built in).
4. **Day 3** — Re-run `research/fetch_1h.py` + `backtest_1h.py` + `robust_E.py` with fresh data; results must match the ranges in section H.
5. **Day 4–5** — Paper trade; check each fill against the real book (`market_impact`) to validate the cost model.
6. **Day 6** — Review intel snapshots; make sure overlaps are being logged with timestamps.
7. **Day 7** — Review: ≥ 3 paper trades executed as specified, cost model error < 10 bps, no runtime gaps > 2 h. Write down any rule you
   had to bend; if any, the week restarts.

**Criteria to authorize limited live trading** (all required): ≥ 20 paper trades; paper profit factor ≥ 1.3; max paper drawdown ≤ 8 %;
no halt triggered; the user explicitly approves in writing. First live size: $60 per position for 2 weeks, then $125.

**Days 8–30 (paper, then possibly limited live)** — track and report weekly:
opportunities detected, qualifying signals, executed trades, win rate, avg win / avg loss, profit factor, net return after fees,
max drawdown, and performance versus two benchmarks: holding USDT (0 %) and a 50 % BTC-USDT / 50 % USDT buy-and-hold.

**30-day scenarios** (assuming ~25 trades at $125, costs included; these are assumptions, not forecasts):

| Scenario | Assumption | Net P&L on $500 | Max drawdown |
|---|---|---|---|
| Pessimistic | edge vanished: mean −0.5 %/trade, 50 % win | ≈ −$16 (−3 %) | 6–8 %; daily-loss/drawdown halts likely bite before −15 % |
| Base | half the backtest: +0.8 %/trade, 60 % win | ≈ +$25 (+5 %) | 4–6 % |
| Optimistic | backtest repeats: +1.6 %/trade, 67 % win | ≈ +$50 (+10 %) | 3–5 % |
| Tail risk | exchange outage / stuck position during a crash; one position −10 % stop, one −20 % gap | −$37 in a day | — |

---

## H. Performance assessment (what was validated, and how)

**Data:** KuCoin 1h candles, 2026-06-28 → 2026-10-31 (3 000 bars) for the 109 USDT pairs with ≥ $0.5 M turnover and ≤ 30 bps spread today;
83 had the full history. Entries at the *next* candle open; costs per symbol = 2 × taker fee + full spread + 0.10 % slippage.
Point-in-time turnover from the candles' own `turnover` field removes most of the "liquid today" selection bias.

**Selected strategy, from `research/results/robust_E.txt`:**

| Test | n | mean | median | win | PF | t-stat |
|---|---|---|---|---|---|---|
| Point-in-time turnover ≥ $3 M (base case) | 322 | +1.63 % | +1.33 % | 67 % | 2.49 | +5.2 |
| ≥ $10 M | 146 | +2.37 % | +1.91 % | 71 % | 3.93 | +4.8 |
| ≥ $1 M | 557 | +1.52 % | +1.28 % | 63 % | 1.95 | +3.8 |
| Excluding top-8 contributing symbols | 245 | +1.31 % | +1.28 % | 65 % | 2.12 | +3.6 |
| Costs doubled | 322 | +1.27 % | +0.99 % | 63 % | 2.05 | +4.1 |
| With 10 % hard stop (the live rule) | 322 | +1.23 % | +1.32 % | 66 % | 1.96 | +4.3 |
| With 3 % stop | 410 | +0.29 % | −0.33 % | 49 % | 1.18 | +1.3 |
| Random entries, same symbols & costs (benchmark) | 1 200 | +0.32 % | −0.34 % | 45 % | 1.18 | +1.2 |
| First half / second half (walk-forward split) | 205 / 205 | +1.40 % / +2.71 % | | 60 % / 69 % | 1.82 / 2.82 | +2.4 / +2.6 |
| BTC 7-day uptrend / downtrend at signal | 139 / 260 | +3.05 % / +1.65 % | | 58 % / 69 % | 2.44 / 2.29 | |

Distribution of the 322 base-case trades: worst −16.4 %, 5th percentile −5.5 %, 25th −1.3 %, 75th +3.7 %, best +51.5 %; 4 trades lost more than 10 %.
Equity simulation at $125/trade, ≤ 2/day: $500 → $717 over 125 days, max drawdown −4.9 %. Mid-liquidity ($1–5 M today) and low-liquidity
pairs show no edge (+0.3 %, PF 1.1): **the effect lives in liquid names only.**

**Caveats that remain (honest list):**
- Four months of data, one market regime (BTC +29 % over the window, with sharp dips). Downtrend-at-signal results are positive, but a
  prolonged bear market is not in sample.
- Monthly results are uneven (July +0.5 %, August +0.2 %, September +3.7 %, October +2.2 %): expect weeks with nothing.
- Symbol selection is still partly "pairs liquid today"; a fully point-in-time universe would require daily symbol snapshots, which the
  scanner now records going forward.
- Fills are modelled at next-open plus costs, not simulated against a replayed book. The paper phase measures real slippage.
- Mean-reversion after capitulation is a well-known effect; its persistence is plausible but not guaranteed, hence the invalidation rules.

**Rejected strategies, evidence:** `research/results/backtest_1h.txt`, `listing_out.txt` (listing study), `poly_leadlag.json`
(correlation of Polymarket price changes with KuCoin BTC log-returns at lags −3…+3 bars across four daily markets: contemporaneous +0.2…+0.46,
no positive lead at +1; at 1-minute resolution BTC→Polymarket lag-1 correlation +0.52, Polymarket→BTC −0.004).

---

## I. Final decision

With the first $500 I would **not trade live yet**. I would: fund the KuCoin account with $500 USDT, hold $250 in reserve, run the hourly
scanner and paper engine for at least 20 trades (~3–4 weeks at the observed signal rate), and compare paper fills with the live book.
If the paper profit factor stays ≥ 1.3 with drawdown ≤ 8 %, authorize live trading at $60 per position on the capitulation-reversion rules
in section E, then $125 after two clean weeks. External "trending" and prediction-market feeds keep logging in the background and get a vote
only once their forward returns prove it. The point of the first $500 is to find out, at a cost of at most $75, whether the measured
edge survives contact with real fills — not to deploy capital because it is there.
