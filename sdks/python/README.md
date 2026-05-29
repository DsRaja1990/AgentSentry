# agentsentry (Python SDK)

Instrument LangChain (and any OpenAI-compatible client) to flow through the
AgentSentry gateway with one URL swap.

## Install

```powershell
pip install -e .
# or with LangChain extras:
pip install -e ".[langchain]"
```

## Use

```python
from agentsentry import Sentry
from agentsentry.langchain import SentryCallbackHandler
from langchain_openai import ChatOpenAI

sentry = Sentry(
    gateway_url="http://localhost:8080",
    api_key="sk_dev_local_demo_key",
    agent_id="support-bot",
)

llm = ChatOpenAI(
    base_url=f"{sentry.gateway_url}/v1/openai/v1",
    api_key="sk-...",                   # real OpenAI key
    callbacks=[SentryCallbackHandler(sentry)],
)
print(llm.invoke("Summarise the company HR policy in one paragraph."))
```

Every call is policy-checked at the gateway, redacted, and logged as a span.

## Out-of-band policy check

```python
d = sentry.policy_check("tool_call", {
    "agent": {"id": "support-bot"},
    "tool":  {"name": "send_email", "args": {"to": "user@external.com"}},
})
if not d.allow:
    raise RuntimeError(d.reason)
```

## Phase 2

OpenAI client wrapper, Anthropic wrapper, Microsoft Agent Framework binding,
async OTLP exporter, automatic prompt-injection scoring.
