"use client";
import { Fragment, useEffect, useMemo, useState } from "react";
import { browserApi, type Span } from "@/lib/browserApi";

const REFRESH_MS = 5_000;

const DECISIONS = ["all", "allow", "deny", "approve", "redact"] as const;
type Decision = typeof DECISIONS[number];

export default function Traces() {
  const [items, setItems]     = useState<Span[]>([]);
  const [error, setError]     = useState<string | null>(null);
  const [q, setQ]             = useState("");
  const [decision, setDec]    = useState<Decision>("all");
  const [agent, setAgent]     = useState<string>("all");
  const [auto, setAuto]       = useState(true);
  const [updated, setUpdated] = useState<Date | null>(null);
  const [open, setOpen]       = useState<Record<string, boolean>>({});
  const [limit, setLimit]     = useState(200);

  const load = async () => {
    try {
      const r = await browserApi.traces(limit);
      setItems(r.items ?? []);
      setError(null);
      setUpdated(new Date());
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  useEffect(() => { load(); }, [limit]);
  useEffect(() => {
    if (!auto) return;
    const id = setInterval(load, REFRESH_MS);
    return () => clearInterval(id);
  }, [auto, limit]);

  const agents = useMemo(() => {
    const s = new Set<string>();
    for (const x of items) if (x.agent_id) s.add(x.agent_id);
    return ["all", ...[...s].sort()];
  }, [items]);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return items.filter(s => {
      if (decision !== "all" && s.policy_decision !== decision) return false;
      if (agent !== "all"    && s.agent_id !== agent)            return false;
      if (!needle) return true;
      return [s.agent_id, s.name, s.model, s.provider, s.policy_id, s.policy_reason, ...(s.guardrail_hits ?? [])]
        .filter(Boolean).join(" ").toLowerCase().includes(needle);
    });
  }, [items, q, decision, agent]);

  const exportCsv = () => {
    const rows = [["ts","agent","kind","name","model","provider","in","out","cost","decision","policy","reason"]];
    for (const s of filtered) {
      rows.push([
        s.ts, s.agent_id, s.kind, s.name, s.model, s.provider,
        String(s.input_tokens), String(s.output_tokens), String(s.cost_usd ?? 0),
        s.policy_decision, s.policy_id ?? "", (s.policy_reason ?? "").replaceAll("\n"," "),
      ]);
    }
    const csv = rows.map(r => r.map(v => `"${(v ?? "").replaceAll('"','""')}"`).join(",")).join("\n");
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const a = document.createElement("a");
    a.href = url; a.download = `traces-${Date.now()}.csv`; a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <>
      <div className="page-head">
        <h2>Traces <span className="muted small">({filtered.length}/{items.length})</span></h2>
        <div className="page-tools">
          <label className="check"><input type="checkbox" checked={auto} onChange={e => setAuto(e.target.checked)} /> live</label>
          <button className="btn" onClick={load}>Refresh</button>
          <button className="btn" onClick={exportCsv}>Export CSV</button>
          <span className="muted small">{updated ? `updated ${updated.toLocaleTimeString()}` : ""}</span>
        </div>
      </div>

      <div className="filters">
        <input className="input" placeholder="search agent / model / policy / reason..." value={q} onChange={e => setQ(e.target.value)} />
        <select className="input" value={decision} onChange={e => setDec(e.target.value as Decision)}>
          {DECISIONS.map(d => <option key={d} value={d}>{d}</option>)}
        </select>
        <select className="input" value={agent} onChange={e => setAgent(e.target.value)}>
          {agents.map(a => <option key={a} value={a}>{a}</option>)}
        </select>
        <select className="input" value={limit} onChange={e => setLimit(Number(e.target.value))}>
          {[100, 200, 500, 1000].map(n => <option key={n} value={n}>{n} rows</option>)}
        </select>
      </div>

      {error && <div className="error">{error}</div>}
      {!error && filtered.length === 0 && <p className="muted">No spans match.</p>}

      {filtered.length > 0 && (
        <table>
          <thead>
            <tr>
              <th></th><th>Time</th><th>Agent</th><th>Kind</th><th>Name / Model</th>
              <th className="num">Tok in/out</th><th className="num">Cost</th><th>Decision</th><th>Policy</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map(s => {
              const expanded = !!open[s.span_id];
              return (
                <Fragment key={s.span_id}>
                  <tr className="row-click" onClick={() => setOpen(o => ({ ...o, [s.span_id]: !expanded }))}>
                    <td>{expanded ? "▾" : "▸"}</td>
                    <td>{new Date(s.ts).toLocaleString()}</td>
                    <td>{s.agent_id}</td>
                    <td>{s.kind}</td>
                    <td>{s.name}{s.model && <span className="muted small"> · {s.model}</span>}</td>
                    <td className="num">{s.input_tokens}/{s.output_tokens}</td>
                    <td className="num">${(s.cost_usd ?? 0).toFixed(4)}</td>
                    <td><span className={`badge badge-${s.policy_decision}`}>{s.policy_decision}</span></td>
                    <td title={s.policy_reason}>{s.policy_id}</td>
                  </tr>
                  {expanded && (
                    <tr className="row-detail">
                      <td colSpan={9}>
                        <div className="detail-grid">
                          <div><span className="kv-k">trace</span><code>{s.trace_id}</code></div>
                          <div><span className="kv-k">span</span><code>{s.span_id}</code></div>
                          <div><span className="kv-k">provider</span>{s.provider}</div>
                          <div><span className="kv-k">duration</span>{s.duration_ms} ms</div>
                          <div className="span-2"><span className="kv-k">reason</span>{s.policy_reason || <em className="muted">—</em>}</div>
                          <div className="span-2"><span className="kv-k">guardrails</span>
                            {(s.guardrail_hits ?? []).length === 0
                              ? <em className="muted">none</em>
                              : s.guardrail_hits.map(h => <span key={h} className="chip">{h}</span>)}
                          </div>
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              );
            })}
          </tbody>
        </table>
      )}
    </>
  );
}
