# `sentry` CLI — Phase 2 stub

> Status: **planned, not yet implemented**.

The CLI will wrap the control-plane REST API for developer ergonomics:

```
sentry login
sentry agent list
sentry agent create --name support-bot --framework langchain
sentry policy push ./policies/block-external-email.rego
sentry policy test ./policies/*.rego --input fixtures/tool_call.json
sentry trace tail --agent support-bot
sentry key create --scope ingest,policy_check
```

Until then, use `curl` against `http://localhost:8081/v1/*` with bearer auth.
Examples are in [docs/usage-developer.md](../docs/usage-developer.md).
