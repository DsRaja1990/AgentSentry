# ADR 0002 — Apache 2.0 open-core licensing

**Status:** Accepted · 2026-05

## Context

Distribution is the hardest problem for an infrastructure product. The
governance market is fragmented and crowded; nobody will adopt a closed-source
proxy in their data path. At the same time, we need a commercial layer to
fund the team.

## Decision

- **Apache 2.0** for the gateway, SDKs, CLI, proto schemas, and a single-tenant
  control-plane build.
- **Commercial license** for the multi-tenant SaaS control plane, RBAC/SSO,
  signed policy bundles, threat-intel feed, compliance evidence packs, and
  enterprise support.

This mirrors Grafana, Sentry (the error tracker), Temporal, and Langfuse —
proven for developer-tools companies.

## Consequences

- Maximises adoption because the data-path component is permissive.
- Avoids the trust loss associated with BSL / SSPL flips.
- We must keep a clean boundary between OSS and commercial code from day one
  (`control/` vs a future `cloud/` directory).
