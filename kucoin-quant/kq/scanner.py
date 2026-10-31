"""Live scanner (read-only). Pulls KuCoin universe, evaluates the strategy on the last CLOSED 1h candle,
checks book impact for the configured position size, logs signals + an intel snapshot, prints a table.
Usage:  python -m kq.scanner [--intel] [--max-symbols N]
"""
from __future__ import annotations
import argparse, json, os, time, datetime as dt
import yaml
from . import kucoin, strategy, intel


def load_cfg(path: str = "config.yaml") -> dict:
    with open(path) as f:
        return yaml.safe_load(f)


def append_jsonl(path: str, row: dict) -> None:
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with open(path, "a") as f:
        f.write(json.dumps(row) + "\n")


def scan(cfg: dict, max_symbols: int = 160, do_intel: bool = False) -> list[dict]:
    syms = kucoin.symbols(); tick_ts, ticks = kucoin.all_tickers()
    st = cfg["strategy"]
    # universe: eligible + today's 24h turnover >= threshold (cheap pre-filter; strategy re-checks point-in-time)
    uni = []
    for sym, meta in syms.items():
        t = ticks.get(sym)
        if not t:
            continue
        ok, why = strategy.is_eligible(sym, meta, t, cfg)
        if ok and float(t["volValue"]) >= st["min_24h_turnover_usd"]:
            uni.append((float(t["volValue"]), sym))
    uni.sort(reverse=True); uni = [s for _, s in uni[:max_symbols]]
    now = int(time.time()); last_closed = now - now % 3600  # evaluate the candle that closed at this ts
    out = []
    for sym in uni:
        cs = kucoin.candles(sym, st["timeframe"], start=last_closed - 80 * 3600, end=last_closed + 1)
        cs = [c for c in cs if c[0] < last_closed]  # drop the still-open candle
        if not cs:
            continue
        t = ticks[sym]; b, a = float(t["buy"]), float(t["sell"]); spread = (a - b) / ((a + b) / 2) * 1e4
        _, taker = kucoin.fee_rates(t)
        sig = strategy.evaluate(sym, cs, cfg, spread, taker)
        if sig:
            imp = kucoin.market_impact(sym, cfg["risk"]["max_position_usd"])
            row = strategy.to_dict(sig); row.update(scan_ts=now, quote_ts=tick_ts, last=float(t["last"]), book=imp, status="candidate")
            if imp["impact_bps"] > 20 or imp["depth_1pct_usd"] < 5 * cfg["risk"]["max_position_usd"]:
                row["status"] = "rejected:liquidity"
            out.append(row); append_jsonl(cfg["paths"]["signals_log"], row)
    if do_intel:
        snap = intel.snapshot(syms); append_jsonl(cfg["paths"]["intel_log"], snap)
        print(f"intel snapshot logged: overlap with KuCoin = {snap['kucoin_overlap']}")
    return out


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--intel", action="store_true"); ap.add_argument("--max-symbols", type=int, default=160)
    ap.add_argument("--config", default="config.yaml"); a = ap.parse_args()
    cfg = load_cfg(a.config)
    rows = scan(cfg, a.max_symbols, a.intel)
    print(f"{dt.datetime.now(dt.UTC).isoformat()}  candidates: {len(rows)}")
    for r in rows:
        print(f"{r['symbol']:14s} close={r['close']:<12g} 24h={r['drop_24h']*100:+.1f}% turn24=${r['turnover_24h']/1e6:.1f}M "
              f"spread={r['spread_bps']:.1f}bps impact={r['book']['impact_bps']:.1f}bps stop={r['stop']:<12g} size=${r['position_usd']} {r['status']}")


if __name__ == "__main__":
    main()
