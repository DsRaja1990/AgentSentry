// Server-side base. In docker compose the UI container reaches control via
// the service DNS name. Outside docker, fall back to localhost so `npm run dev`
// works without extra env wiring.
const API_BASE =
  process.env.CONTROL_API_URL ??
  (process.env.NODE_ENV === "production" ? "http://control:8081" : "http://localhost:8081");
const API_KEY  = process.env.CONTROL_API_KEY ?? "sk_dev_local_demo_key";

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    cache: "no-store",
    headers: {
      "content-type": "application/json",
      "authorization": `Bearer ${API_KEY}`,
      ...(init?.headers ?? {}),
    },
  });
  if (!res.ok) {
    throw new Error(`${path} -> ${res.status}: ${await res.text()}`);
  }
  return (await res.json()) as T;
}

export type Span = {
  trace_id: string; span_id: string; agent_id: string; ts: string;
  duration_ms: number; kind: string; name: string; model: string; provider: string;
  input_tokens: number; output_tokens: number; cost_usd: number;
  policy_decision: string; policy_id: string; policy_reason: string;
  guardrail_hits: string[];
};
export type Agent  = { id: string; name: string; framework: string; owner: string; created_at: string };
export type Policy = { id: string; name: string; language: string; status: string; source: string; version: number; created_at: string };

export const api = {
  health: () => call<{status:string}>("/v1/health"),
  agents:   () => call<{items: Agent[]}>("/v1/agents"),
  policies: () => call<{items: Policy[]}>("/v1/policies"),
  policy:   (id: string) => call<Policy>(`/v1/policies/${id}`),
  traces:   (limit = 100) => call<{items: Span[]}>(`/v1/traces?limit=${limit}`),
};
