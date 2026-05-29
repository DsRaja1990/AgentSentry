# AgentSentry — User Guide

> Plain-language guide for product managers, security and compliance teams,
> and anyone who needs to understand what AgentSentry does without reading code.

## What is AgentSentry?

AgentSentry is a **safety and oversight layer for AI agents**.

If your company uses AI agents — chatbots, copilots, autonomous workflows —
AgentSentry sits between those agents and the world and makes sure they:

- Only do what you've allowed them to do.
- Don't leak sensitive data.
- Don't run up surprise cloud bills.
- Leave a complete, tamper-evident audit trail for regulators.

It works the same way across **Azure, AWS, Google, OpenAI, Anthropic,
LangChain, and Microsoft Agent Framework** — one tool, every agent.

---

## Why you need it

Modern AI agents don't just answer questions. They **take actions**: send
emails, query databases, move money, file tickets, call APIs. Today most
companies have no way to:

- See what their agents are actually doing.
- Block an agent from doing something dangerous *before* it happens.
- Prove to an auditor what an agent did six months ago.

Existing AI tools are slices of the answer. AgentSentry is the **complete
control plane**.

---

## What you get

### 1. A dashboard that shows every agent action

Open <http://localhost:3000> after install. You see:

- **Agents** — every agent registered in your organisation, what framework it
  uses, who owns it.
- **Traces** — every LLM call and tool invocation, with timing, cost, and
  the policy decision that was made.
- **Policies** — the rules currently being enforced and the rules being
  monitored.
- **Audit** — a tamper-evident log suitable for EU AI Act, ISO 42001,
  NIST AI RMF, and SOC 2 evidence requests.

### 2. Guardrails you can change without redeploying

You write rules in plain English-ish syntax:

> *"The support bot may not send email to anyone outside our company."*
>
> *"The finance assistant may never read from the production payroll database."*
>
> *"Any agent action that costs more than \$5 needs a human approval."*

Save the rule. Within ten seconds, every agent in every cloud is enforcing it.
No redeploy. No code change.

### 3. Automatic redaction

If an agent tries to send credit-card numbers, API keys, or personal data,
AgentSentry strips them before the request leaves your network.

### 4. Cost & rate visibility

See spend per agent, per team, per tool. Cap it before it surprises you.

### 5. Compliance evidence on demand

When an auditor asks "what did your AI do for customer X on March 14?",
you click a button and get a signed report.

---

## How it works (no jargon)

1. Your agents talk to AI models (OpenAI, Anthropic, etc.) via the internet.
2. AgentSentry slips in the middle as a **gateway**. Your agents now point at
   AgentSentry instead of straight at the AI provider.
3. Every request passes through. AgentSentry checks it against your rules,
   redacts anything sensitive, records what happened, and (if allowed) sends
   it on.
4. A dashboard shows you everything that happened.

That's it. No agent code changes for most frameworks — just a one-line URL
swap.

---

## Who uses it

| Role | What they do in AgentSentry |
|---|---|
| **Developers** | Point their agents at the gateway URL. Done. |
| **Security teams** | Write and approve policies. Watch the audit log. |
| **Compliance teams** | Export evidence packs. Run access reviews. |
| **Finance** | Watch spend dashboards. Set per-team budgets. |
| **Executives** | Get a one-page weekly summary of agent activity and risk. |

---

## Getting started in 5 minutes

1. **Install** — your platform team runs one Docker command (see the
   [Developer Guide](usage-developer.md)).
2. **Open the dashboard** — <http://localhost:3000>.
3. **Register your first agent** — click *Agents → Add* and give it a name.
4. **Add your first policy** — pick a template from the *Policies → Templates*
   library (e.g. "Block secrets in prompts").
5. **Point one agent at the gateway** — change its API URL to the gateway URL.
6. **Watch traces appear** — every action is now visible.

---

## Frequently asked questions

**Does AgentSentry see my secrets?**
Only momentarily, in memory, in the gateway you run yourself. Secrets are
redacted *before* anything is logged or stored. The gateway is open-source
so you can verify this.

**Does it slow my agents down?**
Less than 5 milliseconds added per call at the 99th percentile. Users will
not notice.

**Which AI providers does it support?**
Today: OpenAI and any OpenAI-compatible API (Azure OpenAI, OpenRouter,
local models via Ollama). Anthropic next. Bedrock, Vertex, Foundry shortly
after.

**Can I run it in my own cloud?**
Yes. AgentSentry is open-source under Apache 2.0. The hosted SaaS control
plane is optional and adds multi-tenancy, SSO, and compliance reports.

**What does it cost?**
The self-hosted version is free. The hosted SaaS is priced per million spans
ingested (similar to Datadog or Langfuse). Enterprise plans add SSO, signed
policy bundles, and dedicated support.

**Is this ready for production?**
The current build is a **vertical-slice MVP** — every layer works end-to-end
but several enterprise features (SPIFFE identity, SSO, .NET/TS SDKs, Helm)
are on the [roadmap](usage-developer.md#10-roadmap-phase-2). Use it for
internal projects today, plan production rollout alongside the roadmap.

---

## Where to go next

- Run the [5-minute install](../README.md#quick-start-5-minutes).
- Tour the dashboard.
- Show this guide to a colleague.
- For technical depth: [Architecture reference](architecture.md).
- For implementation: [Developer guide](usage-developer.md).
