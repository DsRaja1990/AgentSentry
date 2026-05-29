# Install & Configure AgentSentry

This is the only doc you need to stand up AgentSentry. The installer handles
everything that **can** be automated; this guide also lists the few steps that
**must** be done on the agent side (changing the LLM base URL + adding two
headers).

---

## 0 · Prerequisites

| Tool | Version | Why |
|---|---|---|
| Docker | 24+ with the `docker compose` v2 plugin | runs the stack |
| PowerShell 7+ *(Windows)* **or** bash 4+ *(macOS/Linux)* | — | runs the installer |
| 4 GB RAM, 2 CPU, ~3 GB disk | — | postgres + clickhouse + 3 services |

You do **not** need cargo, go, node or python on the host — everything is built
inside containers.

---

## 1 · One-command install

```powershell
# Windows
.\install.ps1
```

```bash
# macOS / Linux
./install.sh
```

The installer is interactive. Defaults (in brackets) are safe — press Enter to
accept them. To skip prompts entirely:

```bash
./install.sh --non-interactive --profile prod
```

### What it asks

| Prompt | Default | What it controls |
|---|---|---|
| Organization name | `Acme` | Shown in audit log + UI title |
| Admin email | `admin@example.com` | Audit log only |
| Enable OpenAI / Anthropic / Gemini / Azure | depends on profile | Which `/v1/<provider>/*` routes the gateway exposes |
| Base URL per provider | provider default | Use this for proxy/private deployments |
| Redaction level (`off`/`standard`/`strict`) | `standard` | Turns PII + secret redaction on/off |
| Starter policy pack (`open`/`standard`/`strict`) | `standard` (dev) / `strict` (prod) | Seeds Rego policies in the control plane |

### What it does — fully automatically

1. Generates two random API keys (`SENTRY_ADMIN_API_KEY`, `SENTRY_TENANT_API_KEY`).
2. Writes [`.env.agentsentry`](../.env.agentsentry) (do **not** commit; in `.gitignore`).
3. Renders [`gateway/config.generated.yaml`](../gateway/config.generated.yaml),
   [`control/config.generated.yaml`](../control/config.generated.yaml),
   and [`deploy/docker-compose.generated.yml`](../deploy/docker-compose.generated.yml).
4. Runs `docker compose up -d --build` with both compose files merged.
5. Polls `/healthz` (gateway) and `/v1/health` (control) until ready.
6. Runs a self-test (`POST /v1/policy/check`).
7. Prints the **tenant API key**, **gateway URL**, and **agent-side steps**.

### What still needs you (the agent developer)

The installer cannot edit your application. Three changes are required per agent:

1. **Change the LLM base URL** to the gateway, keeping the rest of the path:
   | Provider | Base URL |
   |---|---|
   | OpenAI | `http://<gw>/v1/openai/v1` |
   | Anthropic | `http://<gw>/v1/anthropic/v1` |
   | Gemini | `http://<gw>/v1/gemini` |
   | Azure OpenAI | `http://<gw>/v1/azure` |
2. **Add two headers** on every request:
   ```
   x-agentsentry-key:   <SENTRY_TENANT_API_KEY>
   x-agentsentry-agent: <your agent id, e.g. support-bot>
   ```
3. **Keep your real LLM key** as you had it (`Authorization: Bearer …`,
   `x-api-key`, `api-key`, or `?key=` for Gemini). The gateway forwards it.

Copy-paste snippets for **Python / Node / curl / LangChain / `.env`** for each
provider are generated live in the UI under **Onboarding**.

---

## 2 · Verify

```bash
# 1. Tenants + policies are seeded
curl -s http://localhost:8081/v1/agents   -H "Authorization: Bearer $ADMIN_KEY"
curl -s http://localhost:8081/v1/policies -H "Authorization: Bearer $ADMIN_KEY"

# 2. Policy-check directly
curl -s -X POST http://localhost:8080/v1/policy/check \
  -H "x-agentsentry-key: $TENANT_KEY" -H "content-type: application/json" \
  -d '{"package":"tool_call","input":{"tool":{"name":"send_email","args":{"to":"x@gmail.com"}}}}'

# 3. Open the UI
open http://localhost:3000           # macOS
xdg-open http://localhost:3000       # Linux
start http://localhost:3000          # Windows
```

The **status pill** in the UI sidebar shows live control-plane health.

---

## 3 · Day-2 operations

| Task | How |
|---|---|
| Restart the stack | `docker compose -f deploy/docker-compose.dev.yml -f deploy/docker-compose.generated.yml restart` |
| View logs | `docker compose -f deploy/docker-compose.dev.yml -f deploy/docker-compose.generated.yml logs -f gateway control` |
| Re-run installer (idempotent) | `./install.sh --non-interactive` — keeps the existing `.env.agentsentry` if you rename or back it up first |
| Rotate the tenant key | edit `.env.agentsentry`, restart `gateway` + `ui` containers |
| Tear down (keep data) | `docker compose … down` |
| Tear down (drop data) | `docker compose … down -v` |
| Run smoke test | `pwsh scripts/smoke.ps1` or `bash scripts/smoke.sh` |

---

## 4 · Production hardening checklist

The installer's `--profile prod` flips the safe defaults, but the following are
**deployment** concerns you still own:

- [ ] Put a TLS terminator (Caddy / nginx / ALB) in front of `:8080` and `:3000`.
- [ ] Move Postgres + ClickHouse to managed services; set `POSTGRES_DSN` and
      `CLICKHOUSE_DSN` in `.env.agentsentry`.
- [ ] Mount `.env.agentsentry` as a secret (Kubernetes Secret / Azure Key Vault /
      AWS Secrets Manager) — never bake it into an image.
- [ ] Restrict `:8081` (control plane) to your VPC; only the UI and operators
      need it. The public path is `:8080` (gateway) only.
- [ ] Set up scheduled `pg_dump` + ClickHouse backups.
- [ ] Set `SENTRY_SEED_DEMO=false` (the prod profile already does this).
- [ ] Subscribe an alerting destination to the `denied`/`error` metrics in the
      dashboard (export via the **CSV** button on the Traces page until the
      Prometheus exporter ships).

---

## 5 · Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| Installer says `'docker compose' v2 plugin not found` | docker-compose v1 only | Upgrade Docker Desktop / `apt install docker-compose-plugin` |
| Gateway 502s | `SENTRY_GW_*_BASE_URL` empty | Re-run installer and enable the provider |
| UI shows "control plane unreachable" | container died, port collision | `docker compose … logs control` |
| `pol_block_external_email` not denying | regorus rejected the policy at seed-time | `docker compose … logs control` and look for `policy compile error` |
| Self-test passes but no traces appear | telemetry channel is full (10k+ rps burst) | look for `telemetry: channel full` warnings in gateway logs — increase buffer or scale replicas |

---

## 6 · Uninstall

```bash
docker compose -f deploy/docker-compose.dev.yml -f deploy/docker-compose.generated.yml down -v
rm -f .env.agentsentry \
      gateway/config.generated.yaml \
      control/config.generated.yaml \
      deploy/docker-compose.generated.yml
```
