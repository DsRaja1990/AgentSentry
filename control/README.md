# sentry-control (Go)

The AgentSentry control plane. Single static binary serving:

- `POST /v1/ingest`          — span ingest (OTLP-compatible JSON)
- `GET  /v1/health`          — readiness
- `GET  /v1/agents`          — agent registry
- `POST /v1/agents`
- `GET  /v1/policies`        — policy CRUD
- `POST /v1/policies`
- `PUT  /v1/policies/{id}`
- `GET  /v1/policies/{id}`
- `GET  /v1/policies/bundle` — what the gateway pulls every 10s
- `GET  /v1/traces`          — span query
- `POST /v1/api-keys`        — issue scoped keys

## Stores

- **Postgres** (config): tenants, projects, agents, policies, api_keys,
  append-only hash-chained `audit_event`.
- **ClickHouse** (telemetry): `agent_span` partitioned by tenant + month
  with 90-day TTL.

## Build

```powershell
cd control
go build -o bin/sentry-control ./cmd/sentry-control
```

## Migrations

Plain `.sql` files under `migrations/postgres` and `migrations/clickhouse`.
Applied automatically on startup.

## Auth

Bearer token via `Authorization: Bearer ...` or `x-agentsentry-key: ...`.
The dev key `sk_dev_local_demo_key` is accepted in development (configurable
via `SENTRY_DEV_API_KEY=""` to disable). Real keys are hashed with SHA-256
before storage; the raw key is returned **only** on creation.
