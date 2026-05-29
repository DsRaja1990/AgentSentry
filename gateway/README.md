# sentry-gateway (Rust)

Hot-path proxy: MCP + LLM provider interception with inline Rego policy and
PII/secret redaction. Single static binary.

## Layout

```
gateway/
├── Cargo.toml                # workspace
├── config.example.yaml
├── Dockerfile
└── crates/sentry-gateway/
    ├── Cargo.toml
    └── src/
        ├── main.rs           # CLI + tokio runtime
        ├── lib.rs            # re-exports
        ├── config.rs         # YAML + env-var expansion
        ├── identity.rs       # API-key extraction (SPIFFE = Phase 2)
        ├── policy.rs         # regorus-based PolicyStore
        ├── redact.rs         # PII + secret regex scanners
        ├── span.rs           # OTel-GenAI Span shape
        ├── state.rs          # AppState (shared)
        ├── telemetry.rs      # OTLP ingest + policy poller
        └── proxy.rs          # axum router: /v1/openai/* /v1/anthropic/* /v1/mcp
```

## Build & run

```powershell
cargo build --release
.\target\release\sentry-gateway --config config.example.yaml
```

## Inbound headers

| Header | Purpose |
|---|---|
| `x-agentsentry-key`   | AgentSentry API key (gateway auth) |
| `x-agentsentry-agent` | Agent identifier for policy + telemetry |
| `Authorization`       | Provider key (forwarded upstream verbatim) |

The OpenAI SDK Just Works: set `base_url=http://gateway:8080/v1/openai/v1`
and your OpenAI key as usual.

## Tests

```powershell
cargo test
```

## Performance notes

- regorus is a pure-Rust Rego interpreter. We clone the policy bundle and
  build a fresh `Engine` per call for simplicity. Phase 2 will introduce
  bundle compilation + LRU caching to hit the ≤ 5 ms p99 budget under load.
- Streaming responses are not yet bridged — Phase 2 swaps `bytes()` for a
  streaming forward with rolling redaction.
