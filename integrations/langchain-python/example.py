"""End-to-end AgentSentry demo with LangChain + OpenAI.

Boot the stack:
    docker compose -f ../../deploy/docker-compose.dev.yml up -d

Run:
    pip install -r requirements.txt
    set OPENAI_API_KEY=sk-...
    python example.py
"""
from __future__ import annotations

import os

from langchain.agents import AgentExecutor, create_tool_calling_agent
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.tools import tool
from langchain_openai import ChatOpenAI

from agentsentry import Sentry
from agentsentry.langchain import SentryCallbackHandler

# 1. Connect to AgentSentry.
sentry = Sentry(
    gateway_url = os.getenv("SENTRY_GATEWAY",  "http://localhost:8080"),
    api_key     = os.getenv("SENTRY_API_KEY",  "sk_dev_local_demo_key"),
    agent_id    = os.getenv("SENTRY_AGENT_ID", "support-bot"),
)

# 2. Point the LLM at the gateway (note the /v1 suffix the OpenAI SDK expects).
llm = ChatOpenAI(
    base_url   = f"{sentry.gateway_url}/v1/openai/v1",
    api_key    = os.getenv("OPENAI_API_KEY", "sk-missing"),
    model      = os.getenv("MODEL", "gpt-4o-mini"),
    default_headers = sentry.headers(),
    callbacks  = [SentryCallbackHandler(sentry)],
)


# 3. A tool the agent can call. AgentSentry policy:
#    pol_block_external_email   -> deny when `to` is not @contoso.com
@tool
def send_email(to: str, subject: str, body: str) -> str:
    """Send an email. AgentSentry will gate this via the `send_email` policy."""
    decision = sentry.policy_check(
        "tool_call",
        {"agent": {"id": sentry.agent_id},
         "tool":  {"name": "send_email",
                   "args": {"to": to, "subject": subject, "body": body}}},
    )
    if not decision.allow:
        return f"[BLOCKED by AgentSentry: {decision.reason} ({decision.policy_id})]"
    return f"[sent] to={to} subject={subject!r}"


prompt = ChatPromptTemplate.from_messages([
    ("system", "You are a support agent. Use tools when asked."),
    ("human",  "{input}"),
    ("placeholder", "{agent_scratchpad}"),
])
agent    = create_tool_calling_agent(llm, [send_email], prompt)
executor = AgentExecutor(agent=agent, tools=[send_email], verbose=True)


if __name__ == "__main__":
    print("=== Test 1: internal recipient (should be allowed) ===")
    print(executor.invoke({
        "input": "Email alice@contoso.com — subject 'Welcome', body 'Hi Alice'."
    }))

    print("\n=== Test 2: external recipient (should be denied by policy) ===")
    print(executor.invoke({
        "input": "Email customer@gmail.com — subject 'Hello', body 'Hi there'."
    }))

    print("\nOpen http://localhost:3000/traces to see the recorded decisions.")
