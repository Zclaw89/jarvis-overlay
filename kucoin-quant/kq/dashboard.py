"""Writes data/dashboard.html: equity, open positions, last signals, intel overlap. Static, no server needed."""
from __future__ import annotations
import json, os, html, datetime as dt
from .scanner import load_cfg


def main():
    cfg = load_cfg(); P = cfg["paths"]
    def tail(p, n=30):
        return [json.loads(l) for l in open(p)][-n:] if os.path.exists(p) else []
    state = json.load(open(P["state"])) if os.path.exists(P["state"]) else {}
    sig, trades, intel = tail(P["signals_log"]), tail(P["paper_log"]), tail(P["intel_log"], 1)
    rows = lambda xs, keys: "".join("<tr>" + "".join(f"<td>{html.escape(str(x.get(k, '')))}</td>" for k in keys) + "</tr>" for x in xs)
    body = f"""<h1>kucoin-quant paper dashboard</h1><p>{dt.datetime.now(dt.UTC).isoformat()} — equity ${state.get('equity')} halted={state.get('halted')}</p>
<h2>Open</h2><table><tr><th>symbol</th><th>entry_px</th><th>stop</th><th>usd</th></tr>{rows(state.get('open', []), ['symbol','entry_px','stop','usd'])}</table>
<h2>Recent trades</h2><table><tr><th>symbol</th><th>reason</th><th>net_ret</th><th>pnl_usd</th><th>equity</th></tr>{rows([t for t in trades if 'net_ret' in t], ['symbol','reason','net_ret','pnl_usd','equity'])}</table>
<h2>Recent signals</h2><table><tr><th>symbol</th><th>ts</th><th>close</th><th>drop_24h</th><th>status</th></tr>{rows(sig, ['symbol','ts','close','drop_24h','status'])}</table>
<h2>Intel overlap with KuCoin (latest snapshot)</h2><p>{html.escape(str(intel[0].get('kucoin_overlap') if intel else 'none'))}</p>"""
    os.makedirs("data", exist_ok=True)
    open("data/dashboard.html", "w").write(f"<html><head><meta charset='utf-8'><style>body{{font-family:sans-serif;max-width:1000px;margin:auto}}table{{border-collapse:collapse}}td,th{{border:1px solid #ccc;padding:4px 8px}}</style></head><body>{body}</body></html>")
    print("wrote data/dashboard.html")


if __name__ == "__main__":
    main()
