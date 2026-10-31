# $500 Challenge: AI Meme-Coin Intelligence vs. Polymarket Copy Trading

*Research date 2026-10-31 (UTC). Every number comes from scripts in `research/`, run against live public APIs on that date. No trade was placed and nothing connects to an order endpoint.*

## Summary

**Neither strategy showed a copyable edge once delay and costs were applied. My decision is to deploy $0 now, keep the $500 in USDT, and run a free 30-day paper test of Polymarket copying, the less-bad track, before risking up to $100.**

- **Polymarket copying, tested out of sample, loses money.** I picked wallets using data before 1 September and copied their September trades at the price available after a delay. Across 47 wallets and 1,637 trades, copying lost 4.1 % per trade at 60 seconds, 4.9 % at 5 minutes and 11.4 % at 1 hour. The wallets themselves earned only +1.1 % per trade.
- **The impressive leaderboard numbers are mostly measurement errors.** Polymarket's closed-positions endpoint lists winners first by default and omits unredeemed losses. One top wallet shows +$64.3 M realized on closed positions but carries −$54.4 M in losing positions that endpoint leaves out. Run on the same period the leaderboards rank, copying looks like +23 % per trade. Run on a clean period it turns negative.
- **Profitable meme wallets are real, but their profit comes from speed.** I verified three Kolscan wallets on-chain: Sheep +95 SOL, Numer0 +53 SOL and zeropnl +25 SOL realized over 41 hours. Their median hold is 2 to 14 seconds, and 86–87 % of their trades are closed within 60 seconds. Trades held longer than 5 minutes made roughly zero.
- **KuCoin cannot execute either strategy at the needed speed.** KuCoin Predictions reaches Polymarket markets, but it is manual, market-order-only and has no API. Fresh meme coins are not on KuCoin spot, and KuCoin Alpha lists tokens after they are already trending.
- **The video's method is a paid data feed with an AI chat on top.** "Muse" is Meta's AI assistant; the data comes from StalkChain ($29–199 per month). Its "$1000/day" claim could not be verified.


## 1. Video investigation

**Video:** "Is Using Muse AI for Memecoins Basically Cheating? ($1000/DAY)" by Handsome Finance, https://youtu.be/rt5l77b3sX0.
Title and channel are confirmed through YouTube's public oEmbed endpoint.

**I could not watch or transcribe it.** YouTube returned a bot-check page to this cloud environment, and the transcript API was blocked.
I did not try to get around that block. Everything below about the video's content is therefore inferred from public sources, and labelled as such.

| Claim | Status | Basis |
|---|---|---|
| "Muse" is Meta's AI assistant (muse.ai / Muse app) | Verified | StalkChain's setup page for Muse; Meta's own "can Muse make you $1,000" challenge covered by TechRadar and The Next Web |
| The data behind it is StalkChain, connected to Muse as an MCP server | Strong inference | StalkChain publishes a dedicated Muse setup page; the same channel's twin video "Is Using Grok Bot for Memecoins Basically Cheating? ($1000/DAY)" links to StalkChain referral pages; other Handsome Finance videos promote StalkFun / StalkChain |
| The workflow is "ask the AI which tokens 3+ tracked traders bought in the last hour, check the dev and snipers, check a trader's 30d PnL, then buy" | Strong inference | These are the exact example prompts on StalkChain's Muse page |
| Trades are executed in a Solana trading bot or wallet (Axiom, BullX, FOMO, Phantom), not on KuCoin | Strong inference | Handsome Finance video descriptions link Axiom/BullX bots; StalkChain has no execution |
| "$1000/DAY" | **Unverified** | No wallet address or trade log was found; the channel's descriptions carry an "entertainment purposes only" disclaimer and affiliate links |

**What the AI actually adds.** Muse does not see the blockchain. It calls StalkChain tools that return pre-computed data: tracked "KOL" wallet trades, holder lists, buy/sell flow, deployer and sniper checks, exit-slippage quotes, and FOMO-app trader PnL. The AI is a natural-language front end over a paid smart-money database. The edge, if any, lives in the data and in execution speed, not in the language model.

**Actionable workflow extracted (and how this project recreates it):**
1. Maintain a list of tracked wallets with verified history. Here: Kolscan's public leaderboard (free) plus on-chain verification through public Solana RPC.
2. Trigger when several tracked wallets buy the same token inside a short window ("3+ traders aped in the last hour").
3. Filter: deployer has not sold, snipers/bundles not dominant, holder concentration acceptable, liquidity deep enough to exit.
4. Enter fast through a Solana DEX route; exit when the tracked wallets exit or on a time/price stop.
5. The parts that can be improved: measure each wallet's *copyable* return after delay instead of its headline PnL (section 5), and reject the high-frequency wallets whose profits cannot be followed.

## 2. API intelligence (checked 2026-10-31)

"Tested" means I called the endpoint from this environment and got data back. "Docs" means read from the provider's own pages.

### Meme-coin and on-chain data

| Service | Access tested | Cost | What it gives | Verdict for $500 |
|---|---|---|---|---|
| Solana public RPC (publicnode) | Tested: `getSignaturesForAddress`, `getTransaction` work; mainnet-beta returns 429 | Free | Raw truth: every swap a wallet signed | Use. Too slow for famous wallets buried in spam |
| Kolscan leaderboard | Tested: leaderboard page embeds wallet, profit (SOL), wins, losses for 1d/7d/30d | Free | ~150 named KOL wallets with addresses | Use as the candidate list, verify on-chain |
| DexScreener | Tested: search, boosts, profiles, token pairs | Free, 60 req/min on profile endpoints (docs) | Pair price, volume, liquidity, boosts | Use for liquidity and price checks |
| GeckoTerminal | Tested: trending/new pools, token pools, 1-minute OHLCV | Free, about 30 req/min | Minute candles, including pump.fun pools | Use for backtesting entries |
| StalkChain (data behind the video) | Tested: API returns 401 without key. Pricing from its site | $5 trial credit (7 days); $29 / $79 / $199 per month for 1,500 / 5,000 / 16,500 calls | Tracked-trader flows, "3+ traders bought", dev/sniper checks, FOMO trader PnL, MCP for Muse/Claude | Optional, $29 for one month at most, only after the free pipeline shows a signal |
| GMGN | Tested: web API behind Cloudflare (403). OpenAPI CLI has a public demo key "for testing only" | Key from gmgn.ai/ai; price not published | Wallet PnL 1d/7d/30d, smart-money trades, swaps | Not used: no stable free access, and its CLI swaps execute without confirmation |
| Solana Tracker | Tested: 401 without key. Pricing from docs | Free 10,000 req/month at 3 req/s; €50/month for 200k | Wallet PnL, top traders, new tokens | Good free fallback if a key is created |
| Birdeye | Tested: 401 | No free tier found; trackers list $299–449/month | Token and wallet analytics | Too expensive |
| Solscan Pro | Tested: 401 | From $199/month | Indexed history | Too expensive |
| Nansen, Arkham | Nansen root responds; Arkham behind Cloudflare | Paid | Labelled smart money | Too expensive for $500 |
| Pump.fun | Tested: frontend API 403 (Cloudflare); no official public API | — | Launch and bonding-curve data | Use GeckoTerminal/DexScreener/RPC instead |
| StonkFun | No public API found; StalkChain lists StonkFun launch tools | — | Solana launchpad pricing memes against tokenized stocks | Covered only through StalkChain |
| FOMO app | No public API found; StalkChain resolves FOMO handles to wallets | App ~0.5% swap fee (press reports) | Social copy trading | Not an API source |

### Polymarket data

| Endpoint | Tested | Notes that matter |
|---|---|---|
| `data-api.polymarket.com/v2/leaderboard` | Yes | Use `timePeriod=day|week|month|all` and `category=overall|sports|politics|crypto`. The old `lb-api` host is retired. A `window=` parameter is silently ignored |
| `data-api /trades?user=` | Yes | Max 500 per page; offsets stop at about 1,500 rows |
| `data-api /closed-positions?user=` | Yes | **Sorted by realized PnL, largest first, unless `sortBy=TIMESTAMP`.** Capped at about 1,000 rows. Losing positions that were never redeemed are missing |
| `data-api /positions?user=` | Yes | Holds open positions and the unredeemed losers (price 0). Needed for honest PnL |
| `data-api /activity?user=` | Yes | Newest event about 26 seconds old when tested |
| `data-api /trades` (global) | Yes | Newest trade stayed frozen for over a minute and was 150–280 seconds old: cached |
| `gamma-api /markets?condition_ids=` | Yes | Resolution, outcome prices, `feeSchedule.rate`, `orderMinSize` (5 shares) |
| `clob.polymarket.com/prices-history` | Yes | 1-minute history with `startTs/endTs&fidelity=1` |
| `clob.polymarket.com/book` | Yes | Live order book |
| Polygon RPC (publicnode, drpc) | Yes | Chain head 0 seconds old: the fastest way to see a wallet's fills |

Polymarket taker fee per share is `rate × p × (1 − p)`; makers pay nothing. Rates by category from the docs: crypto 0.07, sports 0.05, politics 0.04, geopolitics 0. Many markets carry `feeType: zero_fees`; the simulation reads each market's own rate.

## 3. Profitable meme-coin wallets

**Candidate source.** Kolscan's public leaderboard lists about 150 named KOL wallets with claimed profit in SOL, wins and losses. 41 wallets appear on at least two of the daily, weekly and monthly boards. Those figures are Kolscan's claims.

**Independent check.** I pulled each wallet's own signed transactions from public Solana RPC and rebuilt buys and sells from balance changes. Two limits apply. The free RPC node keeps only about 41 hours of history, and the most famous wallets are buried under spam and failed copy-bot transactions, so they could not be reconstructed for free.

| Wallet | Kolscan 7d / 30d claim (SOL) | On-chain result, last 41 h | Round trips | Win | Median hold | Median size | Profit ex-best trade | Copyable? |
|---|---|---|---|---|---|---|---|---|
| Sheep [`78N177fz…AVkh2`](https://solscan.io/account/78N177fzNJpp8pG49xDv1efYcTMSzo9tPTKEA9mAVkh2) | 99 / 861 | **+95.2 SOL realized** | 55 | 73 % | 2 s | 1.5 SOL | +89.8 SOL | No: 77 % of exits within 30 s |
| Numer0 [`A3W8psib…GpQ2KJ`](https://solscan.io/account/A3W8psibkTUvjxs4LRscbnjux6TFDXdvD4m4GsGpQ2KJ) | 85 / 710 | **+53.4 SOL realized** | 30 | 83 % | 14 s | 2.5 SOL | +20.5 SOL | No: 63 % of exits within 30 s |
| zeropnl [`4xY9T1Q7…duMmr`](https://solscan.io/account/4xY9T1Q7foJzJsJ6YZDSsfp9zkzeZsXnxd45SixduMmr) | 1,356 / 3,553 | **+25.1 SOL realized** | 54 | 61 % | 12 s | 5.1 SOL | +8.9 SOL | No: 71 % of exits within 30 s. Whole wallet history is 41 h, so it is fresh or rotated |
| Wugi [`862TYSvR…RGWo2`](https://solscan.io/account/862TYSvRYoiHAK3F3WwTRYAfuGiQaGdxedN9AGvRGWo2) | 203 / 679 | +12.3 SOL, a single trade | 1 | — | — | 15 SOL | 0 | Too few trades to judge |
| slingoor [`5YRgrP3m…QUzij`](https://solscan.io/account/5YRgrP3mjGzrzirYYN5HAQH19cTYREYwGxW6XRJQUzij) | 5,133 / 24,148 | Not reconstructable: 1,500 successful txs per hour, almost all spam received | — | — | — | — | — | No |
| decu, Cented, trunoest, Kadenox, Schoen | 558 / 3,580 and lower | Not reconstructable: tens of thousands of failed or spam txs per hour | — | — | — | — | — | Unverifiable for free |

**What the copy test shows.** For the three verifiable wallets, the share of round trips already closed before a copier could enter:

| Wallet | Closed within 10 s | within 30 s | within 60 s | within 5 min | Profit from trades held over 5 min |
|---|---|---|---|---|---|
| zeropnl | 49 % | 71 % | 87 % | 96 % | −4.9 SOL |
| Sheep | 66 % | 77 % | 86 % | 96 % | +0.2 SOL |
| Numer0 | 43 % | 63 % | 73 % | 93 % | +6.9 SOL |

These wallets profit from being first: sniping launches and exiting into the buyers who follow them. A copier who sees the trade 30 seconds later is, for most trades, one of those buyers. Minute-level price data from GeckoTerminal could not even resolve their holds; simulated copies came back at exactly the assumed 3 % round-trip cost. The profit is real but belongs to whoever has the fastest execution. That rules out KuCoin's manual routes and most retail copy bots.

### Meme-coin discovery and scoring design

The video's core trigger is "three or more tracked traders bought the same token recently". The scoring below keeps that trigger and adds the checks that separate organic demand from manufactured demand. Every component is computable from free data (public RPC, DexScreener, GeckoTerminal).

| Signal | Source | Organic looks like | Manipulation looks like | Weight in score |
|---|---|---|---|---|
| Distinct verified wallets buying within 30 min | Kolscan list + RPC, signer-only | 3+ wallets with different funding sources | Wallets funded from one address minutes earlier | Gate (must be ≥3) |
| Wallet copyability | Our hold-time measure | Median hold ≥ 10 min | Median hold of seconds (snipers) | Wallets with holds under 2 min are excluded from the count |
| Unique buyers growth (last 15 vs prior 15 min) | GeckoTerminal trades / DexScreener txns | Rising, many small buyers | A few large buys, flat buyer count | +1 if ≥ 1.5× |
| Buy volume acceleration | DexScreener `txns`, `volume.m5/h1` | Steady rise | One spike then nothing | +1 if 5-min volume ≥ 2× 1-hour average |
| Liquidity | DexScreener `liquidity.usd` | ≥ $50k and rising | Falling, or LP not locked/burned | Gate (≥ $50k) |
| Holder concentration | RPC `getTokenLargestAccounts` | Top 10 hold < 30% excluding pool | Top 10 > 50%, bundled snipers | Gate (< 35%) |
| Developer wallet | RPC history of the deployer | No sells, no prior rugs | Dev sold, or rugged before | Gate (no dev sells) |
| Synchronized buys | RPC, same slot / same funder | Spread over minutes | Many buys in one slot from fresh wallets | Reject if > 30% of early volume is same-slot |
| Social momentum | DexScreener boosts/profiles; optional StalkChain theses | Mentions follow price | Paid boosts with no buyers | Informational only until it shows predictive value |

**Do three or more independent indicators help?** That cannot be answered from history with free data, because nobody publishes the past state of these signals at the moment they would have fired. The honest route is a forward test: log every token that passes the gates, then record its price at +1, +5, +15 and +60 minutes from GeckoTerminal. After 100 logged signals, compare the full-score group against tokens that passed only the wallet-cluster gate. Until that comparison exists, the extra indicators are filters that reduce losses from rugs, not a proven source of return.

## 4. Profitable Polymarket wallets

**Method.** I pooled 355 wallets from the official v2 leaderboards: week, month and all-time, across overall, sports, politics and crypto. For each I downloaded closed positions in time order (which includes redeemed losers), open positions (which hold the unredeemed losers at price 0), and up to 1,500 recent trades. Statistics cover only the window where the record is complete.

**Two data traps that inflate every leaderboard-based tool:**
1. `/closed-positions` returns the **largest winners first** unless `sortBy=TIMESTAMP` is passed. With its ~1,000-row cap, a busy wallet's "history" is just its 1,000 best trades. On the profit-sorted data, dozens of top wallets showed a 100 % win rate.
2. Losing positions that were never redeemed **are not in closed positions at all**. They sit in `/positions` with price 0. 112 of the first 134 wallets checked had losers hidden this way. For BreakTheBank, that is 730 losing positions worth −$54.4 M against +$64.3 M of closed winners.

**After correction**, across 230 wallets with complete records: the median win rate is 52 %, 54 wallets lost money over their window, and 74 are high-frequency bots trading over 300 times a day, mostly 5–15 minute crypto markets. 61 wallets pass all filters: net profit positive excluding their 5 best trades, at least 40 positions, t-stat ≥ 2, at least 60 % positive weeks, and a window of at least 30 days. The strongest by t-stat are below. Profile links open their public Polymarket pages.

| # | Wallet (profile) | Type | Window | Positions | Net P&L | Win | Return on cost | Top win share | Max drawdown | Positive weeks | Main markets | Median buy | Sept copy @5 min |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | [02a923d2f6edbc894e](https://polymarket.com/profile/0x2a923d2f6edbc894e76357104e654b27a0d9071e) `0x2a923d2f6edbc894e76357104e654b27a0d9071e` | HFT bot | 352 d | 962 | $536k | 62% | +4.5% | 21% | $-321k | 3/4 | crypto | $91 | not testable |
| 2 | [manwithahat](https://polymarket.com/profile/0x0f1b8f81fdcb47b6c3292b8dbf6c2c16b3c4d367) `0x0f1b8f81fdcb47b6c3292b8dbf6c2c16b3c4d367` | Directional | 479 d | 993 | $309k | 54% | +2.5% | 20% | $-284k | 3/5 | crypto | $37 | not testable |
| 3 | [brockthebaby](https://polymarket.com/profile/0x2adf85be1f25a77ca407cf031bd00d735c05d89c) `0x2adf85be1f25a77ca407cf031bd00d735c05d89c` | HFT bot | 397 d | 963 | $303k | 54% | +8.3% | 6% | $-142k | 1/2 | crypto | $91 | not testable |
| 4 | [coinman2](https://polymarket.com/profile/0x55be7aa03ecfbe37aa5460db791205f7ac9ddca3) `0x55be7aa03ecfbe37aa5460db791205f7ac9ddca3` | Directional | 151 d | 1239 | $433k | 51% | +10.4% | 57% | $-71k | 17/23 | crypto, sports | $25 | not testable |
| 5 | [Apsalar](https://polymarket.com/profile/0xd42f6a1634a3707e27cbae14ca966068e5d1047d) `0xd42f6a1634a3707e27cbae14ca966068e5d1047d` | Directional | 672 d | 951 | $297k | 51% | +6.8% | 20% | $-120k | 35/46 | other, sports | $296 | not testable |
| 6 | [Dropper](https://polymarket.com/profile/0x6bab41a0dc40d6dd4c1a915b8c01969479fd1292) `0x6bab41a0dc40d6dd4c1a915b8c01969479fd1292` | Directional | 466 d | 1020 | $1,263k | 60% | +5.5% | 12% | $-201k | 49/63 | other, geo | $1,061 | not testable |
| 7 | [stonksgoup](https://polymarket.com/profile/0x4a38e6e0330c2463fb5ac2188a620634039abfe8) `0x4a38e6e0330c2463fb5ac2188a620634039abfe8` | Directional | 507 d | 1181 | $523k | 49% | +5.0% | 19% | $-227k | 12/18 | crypto, geo | $26 | not testable |
| 8 | [easypredict](https://polymarket.com/profile/0x388537259dc9e693c1c9b96fdf07a63f6b7aca77) `0x388537259dc9e693c1c9b96fdf07a63f6b7aca77` | Directional | 160 d | 1102 | $98k | 47% | +3.1% | 27% | $-40k | 18/23 | crypto | $11 | not testable |
| 9 | [Tokor](https://polymarket.com/profile/0xa19cbababc312f9df185e49d7004c249ed1ade6b) `0xa19cbababc312f9df185e49d7004c249ed1ade6b` | Directional | 33 d | 971 | $555k | 59% | +37.6% | 82% | $-40k | 5/5 | crypto, other | $101 | -35.3% (n=40) |
| 10 | [0xa95bfdb](https://polymarket.com/profile/0xebf79787ab928c803cbef6fa8e0abe42b9e1da78) `0xebf79787ab928c803cbef6fa8e0abe42b9e1da78` | Directional | 438 d | 988 | $666k | 60% | +6.7% | 13% | $-251k | 2/4 | crypto | $77 | not testable |
| 11 | [sovereign2013](https://polymarket.com/profile/0xee613b3fc183ee44f9da9c05f53e2da107e3debf) `0xee613b3fc183ee44f9da9c05f53e2da107e3debf` | HFT bot | 194 d | 993 | $94k | 51% | +3.8% | 23% | $-70k | 2/2 | sports, other | $43 | not testable |
| 12 | [forgotmydinner](https://polymarket.com/profile/0x4a0eecbf4d18cb6b868f30a7b6e75b7905b9a4b1) `0x4a0eecbf4d18cb6b868f30a7b6e75b7905b9a4b1` | HFT bot | 90 d | 634 | $966k | 31% | +75.7% | 57% | $-21k | 2/8 | other, sports | $400 | not testable |
| 13 | [Kickstand7](https://polymarket.com/profile/0xd1acd3925d895de9aec98ff95f3a30c5279d08d5) `0xd1acd3925d895de9aec98ff95f3a30c5279d08d5` | Directional | 811 d | 1048 | $884k | 56% | +2.4% | 14% | $-197k | 67/100 | sports, politics | $469 | not testable |
| 14 | [paddaa](https://polymarket.com/profile/0x60a92c8620846d81f5ea17b0564e0d4b7c545a71) `0x60a92c8620846d81f5ea17b0564e0d4b7c545a71` | Maker/active trader | 145 d | 997 | $166k | 58% | +4.9% | 13% | $-26k | 17/21 | geo, politics | $527 | not testable |

"Not testable" means the wallet's last 1,500 trades don't reach back before 1 September, so it couldn't enter the clean out-of-sample test. Full results for all 230 wallets are in `research/results/poly_skill3.json`.

**How to read this table.** Long-history directional traders such as Dropper, Apsalar and Kickstand7 earn about 2–7 % on capital per position, with drawdowns of $100–200 k. That is skill relative to other traders, but it is thin. A 1-cent spread on a 50-cent contract costs 2 %. Wallets with returns above 30 % on cost (Tokor, forgotmydinner) get most of it from one or two events: 82 % and 57 % of profit from a single position.

## 5. Copy-trading feasibility

**Simulation rule.** For each BUY by a wallet on a market that has since resolved, we buy at the CLOB price at *their fill time + delay*. We add a 1-cent half-spread and the market's own taker fee (`rate × p × (1 − p)`, rate read per market from Gamma), then hold to resolution. Live books measured on the same day had a median half-spread of 0.5 cents and depth of about $11.6 k within 2 cents, so a $10–50 order adds no measurable slippage. The 1-cent assumption is conservative.

**How fast their trades become visible.** The per-wallet `/activity` endpoint was 26 seconds behind. The global `/trades` feed was cached 150–280 seconds behind. Polygon on-chain logs are real time. A KuCoin user acting on a phone alert adds 2–15 minutes.

**Results, copy return per trade after all costs:**

| Sample | Wallets / trades | Wallet's own return | Copy at 60 s | Copy at 5 min | Copy at 15 min | Copy at 1 h |
|---|---|---|---|---|---|---|
| **Clean out-of-sample: chosen before 1 Sep, copied in Sep** | 47 / 1,637 | +1.1 % (t 0.4) | **−4.1 %** | **−4.9 % (t −1.8)** | −7.7 % (t −3.0) | −11.4 % (t −4.7) |
| Same, top 10 wallets by pre-September record | 10 / 308 | −1.8 % | −7.4 % | −8.5 % | −13.6 % | −15.4 % |
| Random active wallets (benchmark), Sep | 18 / 626 | −11.6 % | −18.2 % | −17.7 % | −2.3 % | −15.0 % |
| *Biased: leaderboard wallets copied in October, the period the boards rank* | 25 / 921 | +30.6 % | +23.4 % | +23.3 % | +22.8 % | +11.6 % |

Correlation between a wallet's pre-September record and its September copy return: **−0.06**. Past performance did not predict copy performance.

**Filters tried on the clean September sample, copy at 5 minutes:**

| Filter | Trades | Mean | Median | Win | t |
|---|---|---|---|---|---|
| All trades | 1,637 | −4.9 % | +3.4 % | 57 % | −1.8 |
| Their size ≥ $1,000 (conviction) | 829 | +0.1 % | +10.0 % | 59 % | 0.0 |
| Their size ≥ $5,000 | 415 | −3.5 % | +10.5 % | 57 % | −0.8 |
| Entry price 0.35–0.65 | 774 | −1.2 % | +39.3 % | 53 % | −0.4 |
| Entry price ≥ 0.85 (near resolution) | 278 | −1.3 % | +3.5 % | 93 % | −0.7 |
| Entry price ≤ 0.25 (longshots) | 240 | −40.4 % | −103.7 % | 14 % | −3.6 |
| Size ≥ $1,000, copy at 60 s (bot speed) | 829 | −0.3 % | +10.4 % | 59 % | −0.1 |

**Conclusions for Part B.**
- No wallet group or filter produced a significantly positive copy return in clean data. The best rule, copying only trades of $1,000 or more, broke even.
- Copying longshots is the fastest way to lose money: −40 % per trade.
- High-frequency bots (5–15 minute crypto markets, sub-$50 tickets, hundreds of trades a day) cannot be copied at any delay you can achieve. Near-resolution buyers earn 1–5 % on 90-cent contracts, which one cent of spread erases.
- What would have to be true to profit: a wallet whose information persists for minutes, trading liquid 35–65 cent markets with size. September shows those wallets exist on average only at breakeven.

**Limits of this test.** The September sample is one month. Only resolved markets are included, so long-dated positions are left out. Wallet selection still comes from current leaderboards, which favours wallets that are still active. A forward paper test is the only way to settle the question for the wallets that were "not testable".

## 6. KuCoin compatibility

| Opportunity | Can KuCoin execute it? | Detail |
|---|---|---|
| Copying a Polymarket wallet | **Partly, manually** | KuCoin **Predictions** (app 5.0.1+, verified accounts in supported regions) routes USDT orders into Polymarket markets. Market orders only; an order cannot be cancelled; not every Polymarket market is listed; fees are shown on the order screen and not published. I found **no Predictions API**, so copying is a phone tap after an alert, which puts you in the 5–15 minute delay rows of section 5 |
| Fresh pump.fun / Solana meme coins (the video's trades) | **No, on the exchange** | Regular KuCoin spot lists tokens days to months after launch. Of 93 spot listings in the last 180 days, the median 24-hour return after listing was negative (listing study in commit 9030876 on this branch) |
| KuCoin **Alpha** zone | **Late and manual** | KuCoin-curated on-chain tokens (Solana and BSC) bought with USDT inside the app. Market orders only, 3 USDT minimum on BSC, no API found. Tokens arrive after they are already trending, which is the opposite of early |
| KuCoin **Web3 Wallet** | **Yes, manually, but it is a self-custody wallet** | KuCoin-branded non-custodial wallet with Solana swaps. It is on-chain execution under the KuCoin name, not exchange execution. No trading API, so it cannot be automated |
| Automated meme-coin copying | **Not through KuCoin** | Needs your own Solana wallet plus a swap route (Jupiter API or a bot such as Axiom/BullX/GMGN). This is the technically correct alternative; it is listed for evaluation only and nothing here connects to it |
| Automated Polymarket copying | **Not through KuCoin** | Needs a direct Polymarket account and CLOB API keys. Polymarket's international site is geo-restricted in several countries, so availability must be checked before it is considered |

**Execution delay this creates.** KuCoin routes are manual for both tracks. Human reaction to a phone alert is realistically 2–15 minutes, so every recommendation below is judged at those delays, not at bot speed.
## 7. The $500 allocation

The evidence does not support putting money at risk today. The allocation therefore starts with nothing deployed, and defines exactly when a small pilot unlocks.

| Item | Amount | Rule |
|---|---|---|
| Live trading now | **$0** | No strategy passed an out-of-sample test |
| Cash reserve (USDT in the KuCoin Funding account) | **$500** | Untouched until a pilot unlocks |
| API and monitoring | **$0 / month** | All sources used are free. StalkChain ($29) is not purchased: it adds no copyable signal at manual speed |
| **Strategy B pilot** (unlocks only after the paper test passes, section 10) | up to **$100** | Executed in KuCoin Predictions |
| Max per position | $10 | One market, one side. Polymarket's 5-share minimum is met at any price above 2 cents |
| Max open positions | 5 | Never two positions in the same event |
| Max loss per position | $10 | A losing binary goes to zero. Size is the stop |
| Daily loss limit | $20 | No new entries for the rest of the UTC day |
| Cumulative drawdown limit | $30 (30 % of the pilot, 6 % of the account) | Pilot stops; back to paper |
| **Strategy A** (meme coins) | **$0** | Research logging only. Execution would need a non-KuCoin wallet, which is your decision to make, and the measured edge is not reachable at copy speed anyway |
| Scaling | Pilot to $200 only after 60 live copies with mean ≥ +3 % and drawdown under $30 | Never more than $200 in the first 90 days |

Why not split the money between tracks: the meme track has a verified profit source that is structurally out of reach, and the Polymarket track has an untested-but-plausible subset (large-conviction trades) that broke even in clean data. Dividing capital would simply fund two negative expectations.

## 8. Technology architecture

Python 3.11, `requests`, a cron or systemd timer, and a SQLite or JSON-lines log. No paid service is required to start.

```
Polymarket copy monitor (primary)                         Meme-coin signal logger (research only)
--------------------------------                          --------------------------------------
shortlist.json (wallets passing the out-of-sample test)   kolscan leaderboard (daily scrape) -> candidate wallets
   │ every 30 s                                              │ every 60 s
   ▼                                                         ▼
data-api /activity?user=W  (≈26 s fresh)                   public Solana RPC: getSignaturesForAddress, signer-only
   or Polygon RPC OrderFilled logs (0–5 s fresh)             │  -> buys/sells per mint from balance deltas
   ▼                                                         ▼
new BUY detected ─► gamma /markets: category, fee rate,    cluster rule: ≥3 verified wallets buy the same mint
   end date, min size                                         within 30 min
   ▼                                                         ▼
clob /book: live ask, depth, slippage for our size         DexScreener/GeckoTerminal: liquidity, age, holders,
   ▼                                                         dev sells, 1-min price path
filter (section 11 rules) ─► reject with reason logged     log signal + price at +1/5/15/60 min (forward test)
   ▼
alert (Telegram bot or email) with market, side, price,
   max price to pay, size in USDT
   ▼
manual order in KuCoin Predictions  ──►  fill logged by hand (price, fee shown)
   ▼
paper/real ledger: our fills vs. their fills, resolution P&L, slippage vs. alert price
```

**Why polling and not WebSockets at first.** The Polymarket CLOB WebSocket streams prices per market, but it does not say *who* traded. Wallet attribution comes from the Data API or from Polygon logs. A 30-second poll of `/activity` for 10–20 wallets is about 40 requests a minute, well inside free limits. Polygon log subscriptions are the upgrade path if the delay analysis shows seconds matter, and it does not for the strategies kept here.

**Open-source bots reviewed.** Several GitHub repos poll the Data API and mirror trades through the CLOB (for example Elshen7/Polymarket-Copy-Bot, LNLUTS/polymarket-copy-trading-bot, PolybaseX, SnipeRun). Many are keyword-stuffed and funnel to paid builds, and all need a private key for live mode. None evaluates whether a trade is still worth copying at the current price, which is the core of this design. Building the small monitor above is simpler and safer than auditing one of them.

**Costs.** $0 per month for APIs. A $5 VPS is optional; the scripts also run on a home PC. StalkChain's $29 tier is the only paid item worth considering, and only for the meme research track.

## 9. Seven-day action plan

| Day | Work | Done when |
|---|---|---|
| 1 | Confirm in the KuCoin app that **Predictions** is available to your account and region (app 5.0.1+, verified KYC). Note the fee it shows on a $10 test ticket without submitting it. Check which of the shortlisted markets it lists | You know the real KuCoin fee and market coverage |
| 1 | Clone this folder; `pip install requests`. Run `research/poly_skill3.py` and `research/poly_oos_report.py oos_sep` to reproduce the tables | Numbers match this report |
| 2 | Build the shortlist: directional or maker wallets from section 4 with median buy ≥ $250 and ≥ 60 % positive weeks (Dropper, Apsalar, Kickstand7, paddaa, plus any that pass after a fresh pull) | `shortlist.json` with 5–15 addresses |
| 2–3 | Write the monitor from section 8: poll `/activity` for each wallet every 30 s, apply the entry rules in section 11, send a Telegram or email alert | Alerts arrive within 60 s of a test wallet's trade |
| 3 | Run the meme logger in parallel: Kolscan list, signer-only RPC parsing, ≥3-wallet cluster rule, prices at +1/5/15/60 min | Every cluster signal is logged with prices |
| 4–7 | **Paper trade Strategy B**: for each alert, record the KuCoin Predictions price you *would* have paid, the time you saw it, and the outcome at resolution | At least 15 paper copies logged with honest timestamps |
| 7 | Review: copy return vs. their return, your real reaction delay, KuCoin vs. Polymarket price gap | Written note; continue to day 30 or stop early if mean < −5 % after 15 trades |

## 10. Thirty-day evaluation

**Measured every week, from the paper log:**

| Metric | Target to continue | Abandon if |
|---|---|---|
| Alerts generated / qualifying copies | ≥ 40 qualifying copies in 30 days | Fewer than 15: too rare to matter |
| Mean copy return per trade, after KuCoin fee and spread | ≥ +3 % | ≤ 0 % after 40 trades |
| t-statistic of copy returns | ≥ 1.5 | — |
| Win rate and average win vs. average loss | Profit factor ≥ 1.2 | Profit factor < 1.0 |
| Your actual reaction delay | Median ≤ 5 min | Median > 15 min (the 15-min row loses 7.7 %) |
| Gap between KuCoin Predictions price and Polymarket CLOB | ≤ 1 cent on average | > 2 cents: KuCoin route is too expensive |
| Paper drawdown on a $100 notional book | ≤ $30 | > $30 |
| Benchmark | Must beat holding USDT (0 %) and the random-wallet benchmark (−17.7 % in September) | — |

**Scenarios for the $100 pilot over 30 trades at $10 each.** These are arithmetic on the measured September rates, not forecasts:

| Scenario | Assumption | Result |
|---|---|---|
| Pessimistic | September all-trades rate, −4.9 % per trade | −$15 (and the $30 stop is never reached unless losses cluster) |
| Base | The ≥ $1,000-conviction filter, +0.1 % per trade | about $0 |
| Optimistic | Shortlist holds up at +5 % per trade | +$15 |

The upside on $100 is small by design. The point of the first 30 days is to learn whether any edge survives manual execution at a cost of at most $30, not to grow the account.

## 11. Final recommendation

**Which strategy first?** Strategy B, Polymarket copying, and only as a **paper test**. Its clean result is a −4.9 % loss per copied trade, better than meme copying, which loses to sub-minute exits, and one subset broke even. Nothing has earned live capital yet.

**Which wallets or signals?** The directional and maker wallets in section 4 with long, consistent records: Dropper `0x6bab41a0dc40d6dd4c1a915b8c01969479fd1292`, Apsalar `0xd42f6a1634a3707e27cbae14ca966068e5d1047d`, Kickstand7 `0xd1acd3925d895de9aec98ff95f3a30c5279d08d5` and paddaa `0x60a92c8620846d81f5ea17b0564e0d4b7c545a71`. Copy only their trades of $1,000 or more. None of these four could enter the September test, because their recent trades don't reach back that far, so the paper test is their first real check. For memes, log the ≥3-verified-wallet cluster signal, but don't trade it.

**Which services and APIs?** Polymarket Data API (`/activity`, `/positions`, `/closed-positions?sortBy=TIMESTAMP`), Gamma (`/markets` for fees and resolution), CLOB (`/book`, `/prices-history`). For memes: public Solana RPC, Kolscan, DexScreener and GeckoTerminal. Total cost $0.

**Where to execute?** KuCoin Predictions, manually, only once the pilot unlocks. Direct Polymarket CLOB execution would be faster, but September showed that speed alone, 60 seconds instead of 5 minutes, only improves the loss from −4.9 % to −4.1 %. It doesn't justify a second platform.

**How much of the $500?** $0 now. Up to $100 after the paper test passes, at $10 per position. $400 stays in reserve regardless.

**Exact entry conditions (paper now, pilot later):**
1. A shortlisted wallet BUYs ≥ $1,000 of one outcome.
2. The market is listed in KuCoin Predictions and resolves within 14 days.
3. The current ask is between 0.35 and 0.80, and no more than 2 cents above their fill.
4. Category is sports, politics or geopolitics. Not 5–15 minute crypto markets.
5. You can place the order within 5 minutes of their fill.

**Exit:** hold to resolution. If the wallet sells half or more of its position before resolution and Predictions lets you sell, sell too.

**Evidence behind the decision:** section 5's clean September test (47 wallets, 1,637 trades), the random-wallet benchmark, the corrected wallet records in section 4, and the on-chain hold times in section 3.

**What stops it:** mean copy return ≤ 0 after 40 paper or live copies; $30 cumulative pilot loss; a median reaction delay over 15 minutes; or a KuCoin price consistently more than 2 cents worse than the Polymarket book.

---
*Reproduce:* `research/poly_wallets.py` → `poly_positions.py` → `poly_closed_ts.py` → `poly_skill3.py`; `poly_oos.py --pre_end 2026-09-01 --post_end 2026-10-01 --out oos_sep` → `poly_oos_report.py oos_sep`; `rand_wallets.py` then `poly_oos.py ... --wallets rand_wallets.json --out oos_rand`. Meme track: `sol_verify.py <wallets…>` → `meme_analyze.py`. Raw downloads are not committed; summaries are in `research/results/`.
