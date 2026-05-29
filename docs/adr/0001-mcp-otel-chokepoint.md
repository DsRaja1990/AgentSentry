# ADR 0001 — MCP + OTel as primary chokepoint

**Status:** Accepted · 2026-05

## Context

To govern AI agents cross-cloud and cross-framework we must intercept their
behaviour at a layer that every framework already uses. The candidates are:

1. Wrap every framework SDK (LangChain, MAF, AutoGen, ADK, Bedrock AgentCore…).
2. Proxy each LLM provider's HTTP API (OpenAI, Anthropic, Bedrock, Vertex,
   Foundry).
3. Intercept the **Model Context Protocol (MCP)** that all major vendors
   (Anthropic, OpenAI, Microsoft, Google) have adopted for tools and resources,
   plus emit / ingest **OpenTelemetry GenAI semconv** spans which the same
   vendors are converging on for observability.

## Decision

Adopt **MCP interception + OpenTelemetry GenAI ingestion** as the primary
chokepoint. SDK hooks are a secondary surface for in-process scenarios that
can't be proxied (on-device agents, frameworks that bypass HTTP).

## Consequences

- Single integration point covers the majority of future agents regardless of
  framework.
- We avoid an arms race chasing every framework's API.
- We become a reference implementation contributor to MCP and OTel GenAI —
  standards leadership becomes a moat.
- We must invest in protocol expertise (MCP versioning, OTLP performance).
