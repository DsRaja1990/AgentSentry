# @agentsentry/sdk (TypeScript) — Phase 2 stub

> Status: **planned, not yet implemented**. Track progress under
> [docs/usage-developer.md §10](../../docs/usage-developer.md#10-roadmap-phase-2).

The TypeScript SDK will mirror the Python SDK API and provide:

- `Sentry` client (gateway URL, API key, agent id)
- Adapters for **Vercel AI SDK** and **LangChain JS**
- An `OpenAI` drop-in that points at the gateway
- A `policyCheck()` helper for in-process enforcement

Until then, point any OpenAI-compatible client at the gateway:

```ts
import OpenAI from "openai";

const openai = new OpenAI({
  baseURL: "http://localhost:8080/v1/openai/v1",
  apiKey:  process.env.OPENAI_API_KEY,
  defaultHeaders: {
    "x-agentsentry-key":   "sk_dev_local_demo_key",
    "x-agentsentry-agent": "support-bot",
  },
});
```

The gateway will enforce policy and emit spans regardless of SDK.
