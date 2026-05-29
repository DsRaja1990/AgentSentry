// Public env for use in CLIENT components -- talks to /api/proxy/* on the
// same Next.js origin. Server components keep using lib/api.ts directly.
"use client";

export type Span = {
  trace_id: string; span_id: string; agent_id: string; ts: string;
  duration_ms: number; kind: string; name: string; model: string; provider: string;
  input_tokens: number; output_tokens: number; cost_usd: number;
  policy_decision: string; policy_id: string; policy_reason: string;
  guardrail_hits: string[];
};
export type Agent  = { id: string; name: string; framework: string; owner: string; created_at: string };
export type Policy = { id: string; name: string; language: string; status: string; source: string; version: number; created_at: string };

async function call<T>(path: string): Promise<T> {
  const r = await fetch(`/api/proxy${path}`, { cache: "no-store" });
  if (!r.ok) throw new Error(`${path} -> ${r.status}: ${await r.text()}`);
  return (await r.json()) as T;
}

export const browserApi = {
  health:   () => call<{status:string}>("/v1/health"),
  agents:   () => call<{items: Agent[]}>("/v1/agents"),
  policies: () => call<{items: Policy[]}>("/v1/policies"),
  traces:   (limit = 200) => call<{items: Span[]}>(`/v1/traces?limit=${limit}`),
};
