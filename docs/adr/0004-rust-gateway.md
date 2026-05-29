# ADR 0004 — Rust for the gateway

**Status:** Accepted · 2026-05

## Context

The gateway sits in every agent's hot path. Our p99 budget is **≤ 5 ms added
latency** for policy + redact. We need:

- Memory safety (the gateway sees secrets and PII).
- High throughput on small per-core footprints (containers, sidecars).
- Static binary, no runtime, predictable GC behaviour.
- A first-class HTTP and TLS stack.

## Decision

Build the gateway in **Rust** using `axum` + `reqwest` + `tokio`. Policy
evaluation uses [regorus](https://github.com/microsoft/regorus) for pure-Rust
Rego. Redaction uses `regex` and a small allow/deny scanner.

Go was the runner-up. Rejected because: garbage collector tail latency,
heavier per-core memory, weaker safety story for a security product.

## Consequences

- Slower initial ramp — fewer Rust contributors than Go.
- Long-term performance ceiling is much higher.
- Single static binary simplifies sidecar / edge deployment.
- We must invest in `cargo deny` / `cargo audit` from day one given the
  product's security posture.
