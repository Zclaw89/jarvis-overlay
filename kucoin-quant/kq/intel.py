"""External intelligence collectors. All endpoints below were verified reachable without a key on 2026-10-31.
They are logged for FORWARD evaluation (signal timestamp -> later KuCoin return). None of them gates a trade
until a logged sample shows predictive value (see PROPOSAL.md, section B and H).

  DexScreener   https://api.dexscreener.com/token-boosts/top/v1, /token-profiles/latest/v1   (docs: 60 req/min, free)
  GeckoTerminal https://api.geckoterminal.com/api/v2/networks/{net}/trending_pools            (free, ~30 req/min)
  CoinGecko     https://api.coingecko.com/api/v3/search/trending                              (demo/free; 429s are common)
  Polymarket    https://gamma-api.polymarket.com/markets, https://clob.polymarket.com/prices-history (free)
  Kalshi        https://api.elections.kalshi.com/trade-api/v2/markets?series_ticker=KXBTC     (free, read)
  KuCoin        /api/v3/announcements (new-listings, delistings)
"""
from __future__ import annotations
import json, time
import requests
from . import kucoin

_S = requests.Session(); _S.headers["User-Agent"] = "kucoin-quant/0.1"


def _get(url: str, params: dict | None = None):
    try:
        r = _S.get(url, params=params, timeout=30)
        if r.status_code != 200:
            return {"_error": r.status_code}
        return r.json()
    except Exception as e:  # network/proxy
        return {"_error": str(e)}


def kucoin_base_map(syms: dict[str, dict]) -> dict[str, str]:
    """base currency -> KuCoin USDT symbol (tradeable only)."""
    return {s["baseCurrency"].upper(): sym for sym, s in syms.items() if s["quoteCurrency"] == "USDT" and s["enableTrading"]}


def dexscreener_boosts() -> list[dict]:
    d = _get("https://api.dexscreener.com/token-boosts/top/v1")
    return d if isinstance(d, list) else []


def dexscreener_token_symbols(tokens: list[dict]) -> list[dict]:
    """Resolve boosted token addresses to symbols/volume via /tokens/v1/{chain}/{addrs} (batches of 30)."""
    out = []
    by_chain: dict[str, list[str]] = {}
    for t in tokens:
        by_chain.setdefault(t["chainId"], []).append(t["tokenAddress"])
    for chain, addrs in by_chain.items():
        for i in range(0, len(addrs), 30):
            d = _get(f"https://api.dexscreener.com/tokens/v1/{chain}/{','.join(addrs[i:i+30])}")
            if isinstance(d, list):
                for p in d:
                    out.append(dict(chain=chain, symbol=(p.get("baseToken") or {}).get("symbol", "").upper(),
                                    vol24=float((p.get("volume") or {}).get("h24") or 0), liq=float((p.get("liquidity") or {}).get("usd") or 0),
                                    chg24=float((p.get("priceChange") or {}).get("h24") or 0)))
            time.sleep(1.1)
    return out


def geckoterminal_trending(network: str = "solana") -> list[dict]:
    d = _get(f"https://api.geckoterminal.com/api/v2/networks/{network}/trending_pools")
    rows = []
    for p in (d.get("data") or []) if isinstance(d, dict) else []:
        a = p["attributes"]
        rows.append(dict(network=network, symbol=a["name"].split(" / ")[0].upper(), vol24=float(a.get("volume_usd", {}).get("h24") or 0),
                         chg24=float((a.get("price_change_percentage") or {}).get("h24") or 0), liq=float(a.get("reserve_in_usd") or 0)))
    return rows


def coingecko_trending() -> list[str]:
    d = _get("https://api.coingecko.com/api/v3/search/trending")
    return [c["item"]["symbol"].upper() for c in d.get("coins", [])] if isinstance(d, dict) else []


def polymarket_crypto(limit: int = 50) -> list[dict]:
    d = _get("https://gamma-api.polymarket.com/markets", dict(limit=limit, active="true", closed="false", tag_id=21, order="volume24hr", ascending="false"))
    rows = []
    for m in d if isinstance(d, list) else []:
        try:
            yes = float(json.loads(m["outcomePrices"])[0])
        except Exception:
            yes = None
        rows.append(dict(question=m.get("question"), slug=m.get("slug"), yes=yes, vol24=float(m.get("volume24hr") or 0), end=m.get("endDate")))
    return rows


def kalshi_btc() -> list[dict]:
    d = _get("https://api.elections.kalshi.com/trade-api/v2/markets", dict(limit=100, status="open", series_ticker="KXBTC"))
    return [dict(ticker=m["ticker"], title=m["title"], yes_bid=m.get("yes_bid"), yes_ask=m.get("yes_ask"), vol24=m.get("volume_24h"))
            for m in (d.get("markets") or [])] if isinstance(d, dict) else []


def kucoin_listings() -> list[dict]:
    try:
        items = kucoin.announcements("new-listings")
    except Exception:
        return []
    return [dict(ts=i["cTime"] // 1000, title=i["annTitle"], spot="Listed on KuCoin" in i["annTitle"] and "Futures" not in i["annTitle"]) for i in items]


def snapshot(syms: dict[str, dict]) -> dict:
    """One timestamped snapshot of every source, cross-referenced to KuCoin USDT pairs."""
    base = kucoin_base_map(syms)
    gt = geckoterminal_trending("solana") + geckoterminal_trending("base") + geckoterminal_trending("bsc")
    cg = coingecko_trending()
    ds = dexscreener_boosts()
    snap = dict(ts=int(time.time()),
                geckoterminal=[dict(r, kucoin=base.get(r["symbol"])) for r in gt],
                coingecko_trending=[dict(symbol=s, kucoin=base.get(s)) for s in cg],
                dexscreener_boosts=[dict(chain=t["chainId"], addr=t["tokenAddress"], amount=t.get("totalAmount")) for t in ds[:30]],
                polymarket=polymarket_crypto(30), kalshi_btc=kalshi_btc()[:10], kucoin_listings=kucoin_listings())
    snap["kucoin_overlap"] = sorted({r["kucoin"] for r in snap["geckoterminal"] + snap["coingecko_trending"] if r.get("kucoin")})
    return snap
