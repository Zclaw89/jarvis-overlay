"""Paper-trading engine. Consumes signals.jsonl, simulates fills at the next 1h open with fees+spread+slippage,
manages stop/time exits from subsequent candles, enforces risk limits, writes paper_trades.jsonl and state.json.
Usage: python -m kq.paper step      # process new signals and close due positions (run hourly via cron)
       python -m kq.paper report    # print equity, stats
"""
from __future__ import annotations
import json, os, sys, time, datetime as dt, statistics as st
from . import kucoin
from .scanner import load_cfg, append_jsonl

SLIPPAGE_BPS = 10


def _load_state(p):
    return json.load(open(p)) if os.path.exists(p) else dict(equity=None, open=[], closed_ids=[], halted=False, day_pnl={}, peak=None)


def _save_state(p, s):
    os.makedirs(os.path.dirname(p) or ".", exist_ok=True); json.dump(s, open(p, "w"), indent=1)


def step(cfg):
    P = cfg["paths"]; R = cfg["risk"]; S = cfg["strategy"]
    state = _load_state(P["state"]); cap = cfg["account"]["capital_usd"]
    state["equity"] = state["equity"] or cap; state["peak"] = state["peak"] or cap
    today = dt.datetime.now(dt.UTC).strftime("%Y-%m-%d")
    # 1. close due positions
    still = []
    for pos in state["open"]:
        cs = kucoin.candles(pos["symbol"], S["timeframe"], start=pos["entry_ts"], end=int(time.time()))
        cs = [c for c in cs if c[0] >= pos["entry_ts"] and c[0] < int(time.time()) - int(time.time()) % 3600]
        exit_px = exit_ts = None; reason = None
        for c in cs:
            if c[4] <= pos["stop"]:
                exit_px, exit_ts, reason = pos["stop"], c[0], "stop"; break
            if c[0] >= pos["entry_ts"] + S["hold_hours"] * 3600:
                exit_px, exit_ts, reason = c[2], c[0], "time"; break
        if exit_px is None:
            still.append(pos); continue
        fee = pos["taker"]; gross = exit_px * (1 - fee - SLIPPAGE_BPS / 2e4) / pos["entry_px"] - 1
        pnl = pos["usd"] * gross
        state["equity"] += pnl; state["day_pnl"][today] = state["day_pnl"].get(today, 0) + pnl
        state["peak"] = max(state["peak"], state["equity"])
        append_jsonl(P["paper_log"], dict(pos, exit_px=exit_px, exit_ts=exit_ts, reason=reason, net_ret=gross, pnl_usd=round(pnl, 2), equity=round(state["equity"], 2)))
    state["open"] = still
    # 2. risk halts
    if state["peak"] - state["equity"] >= R["max_drawdown_usd"]:
        state["halted"] = True
    day_ok = state["day_pnl"].get(today, 0) > -R["daily_loss_limit_usd"]
    # 3. open new positions from fresh signals
    if os.path.exists(P["signals_log"]) and not state["halted"] and day_ok:
        for line in open(P["signals_log"]):
            s = json.loads(line); sid = f"{s['symbol']}:{s['ts']}"
            if s.get("status") != "candidate" or sid in state["closed_ids"] or any(p["id"] == sid for p in state["open"]):
                continue
            if time.time() - s["ts"] > 2 * 3600:  # stale: entry window was the next candle
                state["closed_ids"].append(sid); continue
            if len(state["open"]) >= R["max_concurrent"] or any(p["symbol"] == s["symbol"] for p in state["open"]):
                continue
            nxt = kucoin.candles(s["symbol"], S["timeframe"], start=s["ts"] + 3600, end=s["ts"] + 7200)
            nxt = [c for c in nxt if c[0] == s["ts"] + 3600]
            if not nxt:
                continue  # candle not available yet; try next step
            entry = nxt[0][1] * (1 + s["spread_bps"] / 2e4 + SLIPPAGE_BPS / 2e4)
            taker = (s["est_cost_bps"] - s["spread_bps"] - 10) / 2e4
            entry_eff = entry * (1 + taker)
            usd = min(R["max_position_usd"], state["equity"] * 0.25)
            pos = dict(id=sid, symbol=s["symbol"], entry_ts=nxt[0][0], entry_px=entry_eff, stop=s["stop"], usd=usd, taker=taker, signal_close=s["close"])
            state["open"].append(pos); state["closed_ids"].append(sid)
            append_jsonl(P["paper_log"], dict(pos, event="open"))
    _save_state(P["state"], state)
    print(json.dumps(dict(equity=round(state["equity"], 2), open=[p["symbol"] for p in state["open"]], halted=state["halted"], day_pnl=state["day_pnl"].get(today, 0)), indent=1))


def report(cfg):
    P = cfg["paths"]
    rows = [json.loads(l) for l in open(P["paper_log"])] if os.path.exists(P["paper_log"]) else []
    closed = [r for r in rows if "net_ret" in r]
    if not closed:
        print("no closed paper trades yet"); return
    r = [x["net_ret"] for x in closed]; w = [x for x in r if x > 0]; l = [x for x in r if x <= 0]
    print(f"closed={len(r)} win={len(w)/len(r)*100:.0f}% avgW={st.mean(w)*100 if w else 0:+.2f}% avgL={st.mean(l)*100 if l else 0:+.2f}% "
          f"PF={(sum(w)/abs(sum(l))) if l else float('inf'):.2f} netPnL=${sum(x['pnl_usd'] for x in closed):+.2f} equity=${closed[-1]['equity']}")


if __name__ == "__main__":
    cfg = load_cfg(); {"step": step, "report": report}[sys.argv[1] if len(sys.argv) > 1 else "report"](cfg)
