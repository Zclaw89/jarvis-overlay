"""KuCoin public market-data client (no credentials) and a gated private client.

All public endpoints verified live on 2026-10-31 against https://api.kucoin.com:
  GET /api/v2/symbols                 symbol metadata incl. minFunds, increments, st flag, tradingStartTime
  GET /api/v1/market/allTickers       24h stats + best bid/ask + per-symbol fee rate/coefficient
  GET /api/v1/market/candles          klines, max 1500 per request, newest first
  GET /api/v1/market/orderbook/level2_100   top-100 book
  GET /api/v3/announcements           official announcements (annType=new-listings|delistings)
Rate limit (docs, VIP0): public pool 2000 weight / 30s per IP; spot private pool 4000 / 30s per UID; 429000 on breach.
"""
from __future__ import annotations
import base64, hashlib, hmac, json, os, time, uuid
from typing import Any
import requests

BASE = "https://api.kucoin.com"
_S = requests.Session()
_S.headers["User-Agent"] = "kucoin-quant/0.1 (read-only research)"


def _get(path: str, params: dict | None = None, retries: int = 5) -> Any:
    last = None
    for i in range(retries):
        try:
            r = _S.get(BASE + path, params=params, timeout=30)
            if r.status_code == 429:
                time.sleep(2 ** i); continue
            j = r.json()
            if j.get("code") != "200000":
                raise RuntimeError(f"kucoin {path}: {j}")
            return j["data"]
        except (requests.RequestException, ValueError) as e:  # proxy resets happen; back off
            last = e; time.sleep(2 ** i)
    raise RuntimeError(f"kucoin {path} failed after retries: {last}")


def server_time() -> int:
    return int(_get("/api/v1/timestamp"))


def symbols() -> dict[str, dict]:
    return {s["symbol"]: s for s in _get("/api/v2/symbols")}


def all_tickers() -> tuple[int, dict[str, dict]]:
    d = _get("/api/v1/market/allTickers")
    return int(d["time"]), {t["symbol"]: t for t in d["ticker"]}


def candles(symbol: str, tf: str = "1hour", start: int | None = None, end: int | None = None, max_bars: int = 1500) -> list[list[float]]:
    """Returns ascending [t, open, close, high, low, volume, turnover] (floats). Pages backwards from `end`."""
    sec = {"1min": 60, "5min": 300, "15min": 900, "1hour": 3600, "4hour": 14400, "1day": 86400}[tf]
    end = end or int(time.time())
    start = start or end - max_bars * sec
    out: dict[int, list[float]] = {}
    cur_end = end
    while cur_end > start:
        cur_start = max(start, cur_end - 1500 * sec)
        rows = _get("/api/v1/market/candles", dict(symbol=symbol, type=tf, startAt=cur_start, endAt=cur_end))
        for r in rows:
            out[int(r[0])] = [int(r[0])] + [float(x) for x in r[1:]]
        if not rows:
            break
        cur_end = cur_start
        time.sleep(0.12)
    return [out[k] for k in sorted(out)]


def orderbook(symbol: str) -> dict:
    return _get("/api/v1/market/orderbook/level2_100", dict(symbol=symbol))


def market_impact(symbol: str, usd: float, side: str = "buy") -> dict:
    """Walk the live book for a `usd` market order; returns mid, spread_bps, impact_bps, depth within 1%."""
    ob = orderbook(symbol)
    bids, asks = ob["bids"], ob["asks"]
    best_bid, best_ask = float(bids[0][0]), float(asks[0][0])
    mid = (best_bid + best_ask) / 2
    levels = asks if side == "buy" else bids
    spent = qty = 0.0
    for p, s in levels:
        p, s = float(p), float(s)
        take = min(usd - spent, p * s)
        qty += take / p; spent += take
        if spent >= usd:
            break
    vwap = spent / qty if qty else mid
    depth = sum(float(p) * float(s) for p, s in levels if abs(float(p) / mid - 1) <= 0.01)
    return dict(symbol=symbol, ts=ob["time"], mid=mid, spread_bps=(best_ask - best_bid) / mid * 1e4,
                impact_bps=abs(vwap / mid - 1) * 1e4, depth_1pct_usd=depth, filled_usd=spent)


def announcements(ann_type: str = "new-listings", page_size: int = 50) -> list[dict]:
    return _get("/api/v3/announcements", dict(annType=ann_type, lang="en_US", pageSize=page_size))["items"]


def fee_rates(ticker: dict) -> tuple[float, float]:
    """Effective (maker, taker) from a ticker row: base rate x symbol coefficient (1/2/3 => 0.1%/0.2%/0.3% at VIP0)."""
    return (float(ticker["makerFeeRate"]) * float(ticker["makerCoefficient"]),
            float(ticker["takerFeeRate"]) * float(ticker["takerCoefficient"]))


class PrivateClient:
    """Signed client. Order placement is refused unless config live.enabled, KQ_LIVE=1 and an explicit
    authorization phrase are all present. Keys come only from env: KQ_KEY, KQ_SECRET, KQ_PASSPHRASE."""

    def __init__(self, live_enabled: bool = False):
        self.key = os.environ.get("KQ_KEY"); self.secret = os.environ.get("KQ_SECRET"); self.passphrase = os.environ.get("KQ_PASSPHRASE")
        self.live = bool(live_enabled and os.environ.get("KQ_LIVE") == "1"
                         and os.environ.get("KQ_AUTHORIZED") == "I AUTHORIZE LIVE TRADING")

    def _headers(self, method: str, path: str, body: str = "") -> dict:
        if not (self.key and self.secret and self.passphrase):
            raise RuntimeError("API credentials not set (KQ_KEY/KQ_SECRET/KQ_PASSPHRASE)")
        now = str(int(time.time() * 1000))
        sig = base64.b64encode(hmac.new(self.secret.encode(), (now + method + path + body).encode(), hashlib.sha256).digest()).decode()
        pw = base64.b64encode(hmac.new(self.secret.encode(), self.passphrase.encode(), hashlib.sha256).digest()).decode()
        return {"KC-API-KEY": self.key, "KC-API-SIGN": sig, "KC-API-TIMESTAMP": now, "KC-API-PASSPHRASE": pw,
                "KC-API-KEY-VERSION": "2", "Content-Type": "application/json"}

    def accounts(self) -> list[dict]:  # read-only: balances
        path = "/api/v1/accounts"
        r = _S.get(BASE + path, headers=self._headers("GET", path), timeout=30).json()
        if r.get("code") != "200000": raise RuntimeError(r)
        return r["data"]

    def base_fee(self) -> dict:  # read-only: account's actual maker/taker base rate
        path = "/api/v1/base-fee"
        r = _S.get(BASE + path, headers=self._headers("GET", path), timeout=30).json()
        if r.get("code") != "200000": raise RuntimeError(r)
        return r["data"]

    def place_limit_order(self, symbol: str, side: str, price: str, size: str, post_only: bool = True) -> dict:
        if not self.live:
            raise PermissionError("LIVE TRADING DISABLED: set live.enabled, KQ_LIVE=1 and KQ_AUTHORIZED. Paper mode only.")
        path = "/api/v1/hf/orders"
        body = json.dumps(dict(clientOid=uuid.uuid4().hex, symbol=symbol, side=side, type="limit", price=price, size=size, postOnly=post_only))
        r = _S.post(BASE + path, headers=self._headers("POST", path, body), data=body, timeout=30).json()
        if r.get("code") != "200000": raise RuntimeError(r)
        return r["data"]
