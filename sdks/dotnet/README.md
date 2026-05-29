# AgentSentry.Sdk (.NET) — Phase 2 stub

> Status: **planned, not yet implemented**.

The .NET SDK will provide:

- `SentryClient` (DI-registered) with gateway URL + API key
- Adapters for **Microsoft Agent Framework** and **Semantic Kernel**
- `IPolicyEvaluator` that calls the gateway out-of-band
- `OpenTelemetry` exporter that tags every span with the agent id

Until then, point any OpenAI-compatible .NET client at the gateway and add
the two headers (`x-agentsentry-key`, `x-agentsentry-agent`).

See [docs/usage-developer.md §10](../../docs/usage-developer.md#10-roadmap-phase-2).
