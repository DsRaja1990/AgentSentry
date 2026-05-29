# AgentSentry — Developer Guide

This guide walks you through installing AgentSentry locally, instrumenting an
agent, writing a policy, and shipping it to production.

---

## 1. Prerequisites

| Tool | Version | Used by |
|---|---|---|
| Docker Desktop / Engine | 24+ | Local stack |
| Docker Compose | v2 | Local stack |
| Rust | 1.78+ | Build gateway from source (optional) |
| Go | 1.22+ | Build control plane from source (optional) |
| Node.js | 20+ | Build UI from source (optional) |
| Python | 3.10+ | Python SDK + example |

If you only want to run the stack, **Docker is enough** — everything else is
optional and only needed to hack on AgentSentry itself.

---

## 2. Run the stack

```powershell
git clone <this-repo> AgentSentry
cd AgentSentry
docker compose -f deploy/docker-compose.dev.yml up -d
```

| URL | What |
|---|---|
| <http://localhost:3000> | Dashboard UI |
| <http://localhost:8081/v1/health> | Control plane API |
| <http://localhost:8080> | Gateway (proxy to LLM providers) |

The first start downloads Postgres, ClickHouse, and builds the gateway and
control images. Re-runs are fast.

To wipe state:

```powershell
docker compose -f deploy/docker-compose.dev.yml down -v
```

---

## 3. Get an API key

Every agent needs an API key. For the dev stack a default key is seeded:

```
sk_dev_local_demo_key
```

For real environments, create one via the API:

```powershell
$body = '{"name":"prod-key","scopes":["ingest","policy_check"]}'
curl.exe -X POST http://localhost:8081/v1/api-keys -H "content-type: application/json" -d $body
```

The response includes the raw key **once** — store it as `SENTRY_API_KEY`.

---

## 4. Instrument a Python agent

```powershell
pip install -e sdks/python
```

```python
from agentsentry import Sentry
from agentsentry.langchain import SentryCallbackHandler
from langchain_openai import ChatOpenAI

sentry = Sentry(
    gateway_url="http://localhost:8080",
    api_key="sk_dev_local_demo_key",
    agent_id="support-bot",
)

# Route OpenAI calls through the gateway:
llm = ChatOpenAI(
    base_url=f"{sentry.gateway_url}/v1/openai",
    api_key="sk-...",                        # your real OpenAI key
    callbacks=[SentryCallbackHandler(sentry)],
)

print(llm.invoke("Summarise this PDF in one line."))
```

Every call now flows through the gateway, is policy-checked, and a trace
appears in the dashboard within seconds.

See [integrations/langchain-python/example.py](../integrations/langchain-python/example.py)
for a fuller agent (with tools).

---

## 5. Write a policy

Policies are Rego. Save as `policies/block-external-email.rego`:

```rego
package agentsentry.tool_call

default decision := {"allow": true}

decision := {
    "allow": false,
    "reason": "external recipient",
    "obligations": ["log_to_audit", "notify:#agent-approvals"]
} if {
    input.tool.name == "send_email"
    not endswith(input.tool.args.to, "@contoso.com")
}
```

Upload it:

```powershell
$src = (Get-Content policies/block-external-email.rego -Raw)
$body = @{name="block-external-email"; language="rego"; source=$src; mode="enforce"} | ConvertTo-Json
curl.exe -X POST http://localhost:8081/v1/policies `
  -H "authorization: Bearer sk_dev_local_demo_key" `
  -H "content-type: application/json" -d $body
```

The gateway pulls new policy bundles every 10 seconds. Within that window any
matching tool call is enforced.

---

## 6. Configure the gateway

The gateway reads `config.yaml` (or env vars prefixed `SENTRY_GW_`).
Default config:

```yaml
listen_addr: "0.0.0.0:8080"
control_plane_url: "http://control:8081"
api_key: "${SENTRY_GW_API_KEY}"
poll_interval_seconds: 10
redact:
  enable_pii: true
  enable_secrets: true
upstreams:
  openai:
    base_url: "https://api.openai.com"
  anthropic:
    base_url: "https://api.anthropic.com"
  gemini:
    base_url: "https://generativelanguage.googleapis.com"
  azure:
    base_url: "https://YOUR-RESOURCE.openai.azure.com"
```

Mount your own config in compose:

```yaml
  gateway:
    volumes:
      - ./my-gateway-config.yaml:/etc/agentsentry/config.yaml:ro
```

---

## 7. Available endpoints

### Gateway (data plane)

| Method | Path | Purpose |
|---|---|---|
| ANY | `/v1/openai/*` | Proxy to OpenAI (policy-checked, supports SSE `stream:true`) |
| ANY | `/v1/azure/*` | Proxy to Azure OpenAI (use `?api-version=` and `api-key:` header) |
| ANY | `/v1/anthropic/*` | Proxy to Anthropic (policy-checked, SSE supported) |
| ANY | `/v1/gemini/*` | Proxy to Google Gemini (policy-checked, `:streamGenerateContent` supported) |
| POST | `/v1/mcp` | MCP JSON-RPC proxy (policy-checked) |
| POST | `/v1/policy/check` | Out-of-band policy evaluation |
| GET | `/healthz` | Liveness |

Streaming notes:
- The gateway transparently streams SSE responses chunk-by-chunk; redaction
  runs per chunk (a secret split across two chunks may be missed in MVP).
- Token usage is parsed from the final SSE frame when the upstream provides
  it (OpenAI requires `stream_options: { include_usage: true }`).

### Control plane (REST)

| Method | Path | Purpose |
|---|---|---|
| GET / POST | `/v1/agents` | Agent registry |
| GET / POST / PUT | `/v1/policies` | Policy CRUD |
| GET | `/v1/traces` | Span query (filters: `agent_id`, `since`, `decision`) |
| POST | `/v1/ingest` | OTLP-compatible span ingest (used by gateway/SDK) |
| GET / POST | `/v1/api-keys` | Key management |
| GET | `/v1/health` | Readiness |

OpenAPI is generated from `proto/agentsentry/v1/control.proto`.

---

## 8. Build from source

### Gateway (Rust)

```powershell
cd gateway
cargo build --release
.\target\release\sentry-gateway --config config.example.yaml
```

### Control plane (Go)

```powershell
cd control
go build -o bin/sentry-control ./cmd/sentry-control
.\bin\sentry-control --config config.example.yaml
```

### UI (Next.js)

```powershell
cd ui
pnpm install
pnpm dev
```

---

## 9. Project layout

See [architecture.md §3 and §11](architecture.md#3-components) for the full map.

---

## 10. Roadmap (Phase 2+)

- SPIFFE/SPIRE workload identity (replaces API keys for agents)
- .NET SDK (Microsoft Agent Framework + Semantic Kernel)
- TypeScript SDK (Vercel AI SDK + LangChain JS)
- `sentry` CLI
- Helm chart, Bicep modules, Terraform modules
- Cedar policy engine alongside Rego
- Continuous eval runner, threat-intel feed, policy marketplace
- mTLS gateway ↔ control
- Hash-chain export to customer-owned object storage

Track each in [docs/adr/](adr/) as work begins.

---

## 11. Contributing

1. Fork & branch from `main`.
2. `docker compose -f deploy/docker-compose.dev.yml up -d` for a working stack.
3. Add an ADR under `docs/adr/` for any non-trivial change.
4. PRs must pass `.github/workflows/ci.yml`.
