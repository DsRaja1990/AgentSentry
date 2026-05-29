"""LangChain integration.

Usage:
    from agentsentry import Sentry
    from agentsentry.langchain import SentryCallbackHandler
    from langchain_openai import ChatOpenAI

    sentry = Sentry(gateway_url="http://localhost:8080",
                    api_key="sk_dev_local_demo_key",
                    agent_id="support-bot")

    llm = ChatOpenAI(
        base_url=f"{sentry.gateway_url}/v1/openai/v1",
        api_key="sk-...",
        callbacks=[SentryCallbackHandler(sentry)],
    )

The handler is informational — the gateway is the enforcement point. The
handler enriches local logs and can call `Sentry.policy_check()` for tools.
"""
from __future__ import annotations

from typing import Any
from uuid import UUID

try:
    from langchain_core.callbacks.base import BaseCallbackHandler
except Exception:  # pragma: no cover
    class BaseCallbackHandler:  # type: ignore
        pass

from .client import Sentry


class SentryCallbackHandler(BaseCallbackHandler):
    def __init__(self, sentry: Sentry) -> None:
        self.sentry = sentry

    # LangChain hooks (signatures kept loose; LangChain calls with **kwargs)
    def on_tool_start(self, serialized: dict[str, Any], input_str: str,
                      *, run_id: UUID, **kwargs: Any) -> None:
        tool_name = (serialized or {}).get("name", "unknown")
        decision = self.sentry.policy_check(
            "tool_call",
            {
                "agent":   {"id": self.sentry.agent_id},
                "tool":    {"name": tool_name, "args": {"raw": input_str}},
                "context": {"run_id": str(run_id)},
            },
        )
        if not decision.allow:
            raise PermissionError(
                f"AgentSentry policy denied tool '{tool_name}': "
                f"{decision.reason} (policy={decision.policy_id})"
            )

    def on_llm_start(self, *args: Any, **kwargs: Any) -> None: pass
    def on_llm_end(self,   *args: Any, **kwargs: Any) -> None: pass
    def on_tool_end(self,  *args: Any, **kwargs: Any) -> None: pass
    def on_chain_start(self, *args: Any, **kwargs: Any) -> None: pass
    def on_chain_end(self,   *args: Any, **kwargs: Any) -> None: pass
