# AgentSentry

> Vendor-neutral runtime control plane for AI agents.
> Govern, monitor, and enforce policy across Azure, AWS, Google, LangChain, CrewAI,
> AutoGen, OpenAI Assistants, and Microsoft Agent Framework — from one place.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

AgentSentry is the **"Datadog + Okta + Cloudflare for AI agents"** — an open-core
platform that intercepts every LLM call, tool invocation, and MCP request your
agents make, and applies identity, policy, observability, and audit *at runtime*.

---

## Why

Existing tools cover slices of the problem:

| Layer | Incumbents | What they miss |
|---|---|---|
| Observability | LangSmith, Langfuse, Arize | Log-only; no enforcement |
| Guardrails | Lakera, NeMo, Guardrails AI | Prompt-only; not agent *actions* |
| Enterprise AI GRC | OneTrust, Credo, IBM watsonx | Paperwork; no runtime control |
| Hyperscalers | Bedrock Guardrails, Foundry, Vertex | Vendor-locked to one cloud |

**AgentSentry is the missing piece**: one control plane that governs agent
*actions* (tool calls, MCP invocations, data egress, spend, identity) across
every cloud and framework.

---

## Architecture in one picture

```
┌─────────────────────────────────────────────────────────────┐
│   AgentSentry Control Plane (Go + Next.js UI)               │
│   registry · policy · ingest · traces · audit · dashboards  │
└──────────────▲──────────────────────────────────▲───────────┘
               │ OTLP + policy bundles             │ REST/gRPC
   ┌───────────┴────────────┐         ┌───────────┴───────────┐
   │  Sentry Gateway (Rust) │         │  Sentry SDK (Py/.NET/TS)
   │  MCP + LLM proxy       │         │  in-process hooks      │
   │  OPA policy · redact   │         │  LangChain · MAF · ... │
   └────────────────────────┘         └────────────────────────┘
```

See [docs/architecture.md](docs/architecture.md) for the full reference.

---

## Quick start (5 minutes)

```powershell
git clone <this-repo> AgentSentry
cd AgentSentry
docker compose -f deploy/docker-compose.dev.yml up -d
```

This boots Postgres, ClickHouse, the control plane, the Rust gateway, and the UI.

Open the dashboard:

```
http://localhost:3000
```

Run the end-to-end example:

```powershell
cd integrations/langchain-python
pip install -r requirements.txt
$env:OPENAI_API_KEY = "sk-..."
$env:SENTRY_GATEWAY = "http://localhost:8080"
python example.py
```

You will see traces, policy decisions, and a denied "external email" attempt
appear in the dashboard.

---

## Documentation

- [Architecture reference](docs/architecture.md)
- [Developer guide](docs/usage-developer.md) — install, SDKs, policies, gateway config
- [User guide](docs/usage-user.md) — non-technical overview, dashboard tour
- [Architecture Decision Records](docs/adr/)

---

## Repository layout

```
gateway/        Rust proxy (axum + regorus OPA)
control/        Go control plane (ingest + API + policy + registry)
ui/             Next.js 15 dashboard
sdks/           Python (real), TypeScript / .NET (Phase 2 stubs)
cli/            sentry CLI (Phase 2 stub)
proto/          gRPC / OTel-GenAI schema (source of truth)
deploy/         docker-compose, Helm / Bicep / Terraform stubs
integrations/   LangChain (real), MAF / Bedrock AgentCore stubs
examples/       Runnable demos
docs/           Architecture, ADRs, usage guides
```

---

## Status

This is a working **vertical slice MVP**. Every architectural layer is present
and runs end-to-end via docker-compose. See the [implementation manifest in the
architecture doc](docs/architecture.md#implementation-manifest) for what is
production-real vs. Phase 2 stub.

## License

Apache 2.0 — see [LICENSE](LICENSE).
