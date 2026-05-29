# AgentSentry — Public Contracts

This directory is the **source of truth** for every cross-language contract:

- `agentsentry/v1/common.proto`  — shared types (PolicyDecision, Span, etc.)
- `agentsentry/v1/ingest.proto`  — OTLP-style span ingest into the control plane
- `agentsentry/v1/control.proto` — REST/gRPC control API (agents, policies, traces)

## Versioning

`agentsentry/v1` is **frozen-stable**. Breaking changes require a new
`agentsentry/v2` package and a deprecation cycle.

## Generation

For the MVP we hand-roll types in each language (the Rust gateway uses serde,
the Go control plane uses encoding/json) so the build works out of the box
with no extra toolchain. A `buf generate` setup is on the Phase 2 roadmap;
the `.proto` files here are normative and the JSON shapes used today
match them exactly.

## Canonical JSON shapes

### `Span` (what the gateway emits to `/v1/ingest`)

```json
{
  "trace_id": "0af7651916cd43dd8448eb211c80319c",
  "span_id":  "b7ad6b7169203331",
  "parent_id": "",
  "tenant_id": "t_default",
  "project_id": "p_default",
  "agent_id":   "support-bot",
  "ts":         "2026-05-28T12:34:56.789Z",
  "duration_ms": 412,
  "kind":       "llm",
  "name":       "openai.chat.completions",
  "model":      "gpt-4o-mini",
  "provider":   "openai",
  "input_tokens": 128,
  "output_tokens": 64,
  "cost_usd":   0.0012,
  "tool_name":  "",
  "tool_args_redacted":   "",
  "tool_result_redacted": "",
  "policy_decision": "allow",
  "policy_id":   "pol_default_allow",
  "policy_reason": "",
  "guardrail_hits": [],
  "attributes":  { "agentsentry.sdk": "python@0.1.0" }
}
```

### `PolicyDecision` (what the policy engine returns to the gateway)

```json
{
  "allow": true,
  "require_approval": false,
  "redactions": [],
  "reason": "",
  "policy_id": "pol_…",
  "obligations": []
}
```
