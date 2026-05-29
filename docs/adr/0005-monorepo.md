# ADR 0005 — Monorepo with per-language workspaces

**Status:** Accepted · 2026-05

## Context

AgentSentry spans Rust, Go, Python, TypeScript, C#, and SQL. We can either
split into many repos or keep a monorepo.

## Decision

**Monorepo**, with one workspace file per language ecosystem:

- `Cargo.toml` (Rust workspace) for `gateway/`
- `go.work` for `control/` and future `cli/`
- `pnpm-workspace.yaml` for `ui/` and `sdks/typescript/`
- `pyproject.toml` per Python package under `sdks/python/`
- `*.csproj` per .NET package under `sdks/dotnet/`

`proto/` is the cross-language source of truth — generated code is checked
in per consumer.

## Consequences

- Atomic cross-cutting changes (proto + gateway + SDK) in one PR.
- Heavier CI matrix — managed via path filters in `.github/workflows/ci.yml`.
- New contributors can clone one thing and run `docker compose up`.
