# ADR 0003 — Rego primary, Cedar secondary

**Status:** Accepted · 2026-05

## Context

A policy engine is the heart of the gateway. Options:

- **Rego (OPA)** — CNCF standard, ubiquitous in Kubernetes, huge community.
  Pure-Rust interpreter [regorus](https://github.com/microsoft/regorus) avoids
  CGo / WASM toolchain in the gateway.
- **Cedar (AWS)** — newer, formally verified, simpler, gaining traction inside
  AWS shops.
- Custom DSL — rejected: zero ecosystem, hostile to enterprise buyers.

## Decision

Support **Rego first** (regorus, in-process). Add **Cedar** as a second
backend in Phase 2 for AWS-heavy customers. Both produce the same
`PolicyDecision` JSON the gateway acts on.

## Consequences

- Day-one compatibility with existing OPA bundles, conftest tests, and skills.
- No CGo in the gateway — single static Rust binary.
- A small adapter layer (`policy::Engine` trait) keeps Cedar a clean add-on.
