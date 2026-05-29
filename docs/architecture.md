# AgentSentry — Architecture Reference

> Version 0.1 · Source of truth for all components

## 1. Vision

Vendor-neutral runtime control plane for AI agents. Governs **agent actions**
(LLM calls, tool invocations, MCP requests, data egress, spend) across Azure,
AWS, Google, LangChain, CrewAI, AutoGen, OpenAI Assistants, and Microsoft
Agent Framework.

The unsolved problem we attack: **no incumbent governs agent actions inline,
cross-cloud, with one policy language and one audit log.**

## 2. System context

```
┌─────────────────────────────────────────────────────────────┐
│   AgentSentry Control Plane (SaaS, multi-tenant)            │
│   - Agent registry, policy authoring, audit, eval, dashboards│
└──────────────▲──────────────────────────────────▲───────────┘
               │ OTel + policy sync                │
   ┌───────────┴────────────┐         ┌───────────┴───────────┐
   │  Sentry Gateway        │         │  Sentry SDK / Sidecar │
   │  (MCP + LLM proxy)     │         │  (in-process hooks)   │
   │  - Self-host or hosted │         │  - LangChain, MAF,    │
   │  - Inline enforcement  │         │    Bedrock AgentCore  │
   │  - Tool-call inspection│         │                       │
   └────────────────────────┘         └───────────────────────┘
```

**Principle:** the gateway is the only thing that can BLOCK. SDKs observe and
may *ask* the control plane for a decision when no inline proxy is feasible.

## 3. Components

| Component | Language | Responsibility | State |
|---|---|---|---|
| `sentry-gateway` | Rust (axum + reqwest + regorus) | MCP/LLM reverse-proxy, inline policy, redact, OTLP emit | Stateless (pulls policy bundles) |
| `sentry-control` | Go (chi + pgx + clickhouse-go) | Ingest + REST API + policy + registry | PG + ClickHouse |
| `ui` | Next.js 15 / React 19 | Dashboards, policy authoring, audit viewer | Stateless |
| `agentsentry` (Python SDK) | Python 3.10+ | LangChain callback, OpenAI client wrapper | Stateless |
| `AgentSentry.Sdk` (.NET) | C# 12 / .NET 8 | Microsoft Agent Framework / SK integration | Stateless (Phase 2 stub) |
| `@agentsentry/sdk` (TS) | TypeScript | Vercel AI SDK / LangChain JS (Phase 2 stub) |
| `sentry` CLI | Go | `sentry init`, `sentry policy test` (Phase 2 stub) |

## 4. Data model

### Postgres (configuration)

```sql
tenant(id, name, plan, created_at)
project(id, tenant_id, name, created_at)
agent(id, project_id, name, framework, owner, identity_kind, identity_ref, created_at)
api_key(id, tenant_id, hashed_key, scopes, created_at, revoked_at)
policy(id, project_id, name, language, source, version, status, created_at)
policy_binding(id, policy_id, scope, target_id, mode)  -- enforce | monitor
audit_event(seq, prev_hash, hash, tenant_id, actor, action, target, payload, ts)
```

The `audit_event` table is append-only and hash-chained — every row's `hash`
covers `prev_hash`, providing tamper-evident evidence for EU AI Act / SOC 2.

### ClickHouse (telemetry)

```sql
agent_span(
  trace_id, span_id, parent_id,
  tenant_id, project_id, agent_id,
  ts, duration_ms,
  kind,                -- llm | tool | mcp | guardrail | policy
  name, model, provider,
  input_tokens, output_tokens, cost_usd,
  tool_name, tool_args_redacted, tool_result_redacted,
  policy_decision,     -- allow | deny | approve | redact
  policy_id, policy_reason,
  guardrail_hits Array(String),
  attributes Map(String,String)
)
```

Schema is **OpenTelemetry GenAI semconv** with an `agentsentry.*` namespace
for policy and guardrail attributes.

## 5. Policy model

Two policy languages, same evaluation pipeline:

- **Rego (OPA)** — primary. Evaluated in the gateway via the
  [regorus](https://github.com/microsoft/regorus) pure-Rust interpreter
  (no CGo, no WASM toolchain).
- **Cedar** — secondary (Phase 2).

Example policy (Rego):

```rego
package agentsentry.tool_call

default decision := {"allow": true}

decision := {
  "allow": false,
  "reason": "external email recipient",
  "obligations": ["log_to_audit"]
} if {
  input.tool.name == "send_email"
  not endswith(input.tool.args.to, "@contoso.com")
}
```

Decision schema:

```json
{
  "allow": false,
  "require_approval": false,
  "redactions": [{"path": "args.body", "kind": "pii_email"}],
  "reason": "external_recipient",
  "policy_id": "pol_…",
  "obligations": ["log_to_audit"]
}
```

Policy bundles are signed (cosign — Phase 2) and pulled by the gateway every
10 s. Decisions are emitted as spans for full traceability.

## 6. Request lifecycle (LLM call hot path)

1. Agent app issues OpenAI-compatible request to the gateway.
2. Gateway authenticates the workload (API key today, SPIFFE SVID in Phase 2),
   resolves `agent_id`.
3. Gateway runs pre-call redaction (PII / secret scanners).
4. Gateway evaluates policy via in-process regorus — **no network hop**.
5. On `allow`, request is forwarded to the upstream provider; on `deny`, gateway
   returns a 403-style structured error.
6. Gateway runs post-call redaction on the response.
7. Gateway emits one `agent_span` per call to the control plane via OTLP/HTTP.

**Latency budget:** p50 ≤ 2 ms, p99 ≤ 5 ms for policy + redact (excluding
upstream LLM call).

## 7. Identity

- **Humans →** OIDC (Entra ID / Okta / Google) — Phase 2.
- **Agents (workloads) →** SPIFFE SVIDs (Phase 2). MVP uses bearer API keys
  scoped per agent and stored hashed (`bcrypt`) in Postgres.

Every audit event carries the subject — non-repudiation by design.

## 8. Public contracts (frozen in `proto/`)

- **OTLP ingest** — OpenTelemetry GenAI semconv + `agentsentry.*` extensions
- **Control REST API** — `/v1/agents`, `/v1/policies`, `/v1/traces`, `/v1/ingest`
- **SDK contract** — every SDK exposes `init`, `instrument`, `policy_check`, `flush`

SemVer per surface; `proto/agentsentry/v1` is the stable contract.

## 9. Security posture

- mTLS between gateway and control plane — Phase 2 (HTTP + bearer in MVP).
- Tenant isolation: Postgres RLS, ClickHouse partition-per-tenant.
- Secrets never reach traces — redactor runs *before* OTLP export.
- Audit log: append-only, hash-chained; nightly export to customer-owned
  S3/Blob (BYO bucket) on the enterprise tier — Phase 2.
- Threat model: `docs/adr/0002-threat-model.md` — Phase 2.

## 10. Non-functional targets

| Metric | Target |
|---|---|
| Gateway p99 added latency | ≤ 5 ms |
| Gateway throughput / core | ≥ 10 k req/s |
| Ingest sustained | 100 k spans/s per shard |
| Policy bundle propagation | < 10 s globally |
| Dashboard query p95 | < 1.5 s for 30-day window |
| SaaS availability | 99.9 % (Phase 2), 99.95 % (Phase 3) |

## 11. Implementation manifest

**Real (runs end-to-end via docker-compose):**

- Rust gateway with OpenAI-compatible LLM proxy, regorus policy, redaction,
  OTLP-over-HTTP export.
- Go control plane single binary: ingest + REST API + policy + registry.
- Postgres + ClickHouse schemas and migrations.
- Next.js 15 dashboard: traces, agents, policies, dashboard pages.
- Python SDK with LangChain callback handler.
- End-to-end example: LangChain agent → gateway → OpenAI with one Rego policy.
- `docker-compose.dev.yml` wires it all together.

**Stub (README + scaffold, marked Phase 2):**

- SPIFFE / SPIRE identity (API-key MVP today).
- .NET SDK, TypeScript SDK, `sentry` CLI.
- Helm chart, Bicep, Terraform.
- Eval runner, threat-intel feed, policy marketplace.

## 12. Architecture Decision Records

| ID | Title |
|---|---|
| [0001](adr/0001-mcp-otel-chokepoint.md) | MCP + OTel as primary chokepoint |
| [0002](adr/0002-apache-open-core.md) | Apache 2.0 open-core licensing |
| [0003](adr/0003-rego-cedar.md) | Rego primary, Cedar secondary |
| [0004](adr/0004-rust-gateway.md) | Rust for the gateway |
| [0005](adr/0005-monorepo.md) | Monorepo with per-language workspaces |
