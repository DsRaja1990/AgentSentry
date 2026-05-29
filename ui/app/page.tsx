"use client";
import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { browserApi, type Span } from "@/lib/browserApi";
import Sparkline from "./components/Sparkline";

const REFRESH_MS = 10_000;

function denyBuckets(spans: Span[], buckets = 24): number[] {
  if (spans.length === 0) return new Array(buckets).fill(0);
  const now = Date.now();
  const windowMs = 60 * 60 * 1000; // last hour
  const step = windowMs / buckets;
  const start = now - windowMs;
  const out = new Array(buckets).fill(0);
  for (const s of spans) {
    if (s.policy_decision !== "deny") continue;
    const t = new Date(s.ts).getTime();
    if (t < start || t > now) continue;
    const idx = Math.min(buckets - 1, Math.max(0, Math.floor((t - start) / step)));
    out[idx]++;
  }
  return out;
}

export default function Dashboard() {
  const [agents, setAgents]     = useState<number>(0);
  const [policies, setPolicies] = useState<number>(0);
  const [spans, setSpans]       = useState<Span[]>([]);
  const [error, setError]       = useState<string | null>(null);
  const [auto, setAuto]         = useState(true);
  const [updated, setUpdated]   = useState<Date | null>(null);

  const load = async () => {
    try {
      const [a, p, t] = await Promise.all([browserApi.agents(), browserApi.policies(), browserApi.traces(500)]);
      setAgents(a.items?.length ?? 0);
      setPolicies(p.items?.length ?? 0);
      setSpans(t.items ?? []);
      setError(null);
      setUpdated(new Date());
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  useEffect(() => { load(); }, []);
  useEffect(() => {
    if (!auto) return;
    const id = setInterval(load, REFRESH_MS);
    return () => clearInterval(id);
  }, [auto]);

  const denyCount = useMemo(() => spans.filter(s => s.policy_decision === "deny").length, [spans]);
  const cost      = useMemo(() => spans.reduce((a, s) => a + (s.cost_usd ?? 0), 0), [spans]);
  const tokensIn  = useMemo(() => spans.reduce((a, s) => a + (s.input_tokens ?? 0), 0), [spans]);
  const tokensOut = useMemo(() => spans.reduce((a, s) => a + (s.output_tokens ?? 0), 0), [spans]);
  const denyRate  = spans.length ? Math.round((denyCount / spans.length) * 1000) / 10 : 0;
  const series    = useMemo(() => denyBuckets(spans), [spans]);
  const topDenies = useMemo(() => {
    const m = new Map<string, number>();
    for (const s of spans) if (s.policy_decision === "deny") m.set(s.policy_id || "(unknown)", (m.get(s.policy_id || "(unknown)") ?? 0) + 1);
    return [...m.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5);
  }, [spans]);
  const recent = useMemo(() => spans.slice(0, 8), [spans]);

  return (
    <>
      <div className="page-head">
        <h2>Dashboard</h2>
        <div className="page-tools">
          <label className="check"><input type="checkbox" checked={auto} onChange={e => setAuto(e.target.checked)} /> auto-refresh</label>
          <button className="btn" onClick={load}>Refresh</button>
          <span className="muted small">{updated ? `updated ${updated.toLocaleTimeString()}` : ""}</span>
        </div>
      </div>

      {error && <div className="error">Control plane unreachable: {error}</div>}

      <div className="cards">
        <div className="card">
          <div className="label">Agents</div>
          <div className="value">{agents}</div>
        </div>
        <div className="card">
          <div className="label">Policies</div>
          <div className="value">{policies}</div>
        </div>
        <div className="card">
          <div className="label">Spans (last 500)</div>
          <div className="value">{spans.length}</div>
          <div className="sub muted">{tokensIn.toLocaleString()} in / {tokensOut.toLocaleString()} out</div>
        </div>
        <div className="card">
          <div className="label">Denied</div>
          <div className="value" style={{ color: "var(--red)" }}>{denyCount}</div>
          <div className="sub muted">{denyRate}% deny rate</div>
        </div>
        <div className="card">
          <div className="label">Cost (USD)</div>
          <div className="value">${cost.toFixed(3)}</div>
        </div>
        <div className="card card-wide">
          <div className="label">Denies — last 60 min</div>
          <Sparkline data={series} stroke="#ef4444" width={420} height={56} />
        </div>
      </div>

      {spans.length === 0 && !error && (
        <div className="panel onboarding-card">
          <h3>No telemetry yet</h3>
          <p className="muted">
            Point an agent at the gateway to start seeing live traces, cost, and policy hits here.
          </p>
          <Link className="btn btn-primary" href="/onboarding">Open onboarding guide →</Link>
        </div>
      )}

      <div className="two-col">
        <div className="panel">
          <h3>Top deny policies</h3>
          {topDenies.length === 0 ? (
            <p className="muted small">Nothing denied recently.</p>
          ) : (
            <table className="mini">
              <tbody>
                {topDenies.map(([id, n]) => (
                  <tr key={id}><td><code>{id}</code></td><td className="num">{n}</td></tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
        <div className="panel">
          <h3>Recent activity</h3>
          {recent.length === 0 ? (
            <p className="muted small">No spans yet.</p>
          ) : (
            <table className="mini">
              <tbody>
                {recent.map(s => (
                  <tr key={s.span_id}>
                    <td className="muted small">{new Date(s.ts).toLocaleTimeString()}</td>
                    <td>{s.agent_id}</td>
                    <td>{s.model || s.name}</td>
                    <td><span className={`badge badge-${s.policy_decision}`}>{s.policy_decision}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </>
  );
}
