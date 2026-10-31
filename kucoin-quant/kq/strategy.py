"""Primary strategy: *liquid capitulation reversion* (long-only spot, 1h candles).

Signal at the close of 1h candle i for symbol s (all values point-in-time, no look-ahead):
  1. trailing 24h KuCoin turnover >= min_24h_turnover_usd      (liquidity, measured on KuCoin itself)
  2. close[i] < min(low[i-48 .. i-2])                           (new 48h low)
  3. close[i] < close[i-24] * (1 - min_24h_drop)                (>= 5% down in 24h: capitulation, not drift)
  4. symbol not stablecoin / not 'st' flagged / not listed in the last 14 days / spread <= max_spread_bps
Execution: buy at the OPEN of candle i+1 (post-only limit at best bid, re-pegged for 3 minutes, else skip).
Exit: close of candle i+1+hold_hours (24h) OR hard stop 10% below entry, whichever first. One trade per symbol per 24h.
Backtest evidence: research/results/backtest_1h.txt (n=322, Jun-Oct 2026, net of fees+spread+0.1% slippage).
"""
from __future__ import annotations
import time
from dataclasses import dataclass, asdict


@dataclass
class Signal:
    symbol: str
    ts: int            # candle close timestamp (s)
    close: float
    drop_24h: float
    turnover_24h: float
    low_48h: float
    spread_bps: float
    entry_ref: float   # reference entry = signal close (actual fill = next open / limit)
    stop: float
    target_note: str
    position_usd: float
    est_cost_bps: float


def is_eligible(sym: str, meta: dict, ticker: dict, cfg: dict, now: float | None = None) -> tuple[bool, str]:
    st = cfg["strategy"]; now = now or time.time()
    if meta.get("quoteCurrency") != "USDT" or not meta.get("enableTrading"):
        return False, "not USDT/trading"
    if any(meta["baseCurrency"].upper().startswith(p) for p in st["excluded_prefixes"]):
        return False, "stable/fiat"
    if st["exclude_st_flag"] and meta.get("st"):
        return False, "st-flag"
    tst = meta.get("tradingStartTime")
    if tst and now - tst / 1000 < st["exclude_listed_within_days"] * 86400:
        return False, "recent listing"
    try:
        b, a = float(ticker["buy"]), float(ticker["sell"])
    except (TypeError, ValueError, KeyError):
        return False, "no quote"
    if b <= 0:
        return False, "no bid"
    if (a - b) / ((a + b) / 2) * 1e4 > st["max_spread_bps"]:
        return False, "spread"
    return True, "ok"


def evaluate(symbol: str, cs: list[list[float]], cfg: dict, spread_bps: float, fee_taker: float, i: int | None = None) -> Signal | None:
    """cs: ascending [t,o,c,h,l,vol,turnover]; evaluate candle i (default: last CLOSED candle)."""
    st = cfg["strategy"]; lb = st["lookback_hours"]
    i = len(cs) - 1 if i is None else i
    if i < lb + 1:
        return None
    t, o, c, h, l, v, turn = cs[i]
    turn24 = sum(x[6] for x in cs[i - 24:i])
    if turn24 < st["min_24h_turnover_usd"]:
        return None
    low48 = min(x[4] for x in cs[i - lb:i - 1])
    if not (c < low48 and c < cs[i - 24][2] * (1 - st["min_24h_drop"])):
        return None
    cost_bps = (2 * fee_taker) * 1e4 + spread_bps + 10
    return Signal(symbol=symbol, ts=int(t), close=c, drop_24h=c / cs[i - 24][2] - 1, turnover_24h=turn24, low_48h=low48,
                  spread_bps=spread_bps, entry_ref=c, stop=round(c * (1 - st["stop_loss"]), 10),
                  target_note=f"time exit after {st['hold_hours']}h (no fixed target; backtest median +1.3%)",
                  position_usd=cfg["risk"]["max_position_usd"], est_cost_bps=cost_bps)


def to_dict(s: Signal) -> dict:
    return asdict(s)
