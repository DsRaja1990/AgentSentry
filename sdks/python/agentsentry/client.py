"""AgentSentry client — connection settings + out-of-band policy check."""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

import httpx


@dataclass
class PolicyDecision:
    allow: bool = True
    require_approval: bool = False
    redactions: list[dict[str, Any]] = field(default_factory=list)
    reason: str = ""
    policy_id: str = ""
    obligations: list[str] = field(default_factory=list)

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "PolicyDecision":
        return cls(
            allow            = bool(d.get("allow", True)),
            require_approval = bool(d.get("require_approval", False)),
            redactions       = list(d.get("redactions") or []),
            reason           = str(d.get("reason", "")),
            policy_id        = str(d.get("policy_id", "")),
            obligations      = list(d.get("obligations") or []),
        )


@dataclass
class Sentry:
    """Holds connection details for the AgentSentry gateway and control plane.

    Pass the gateway URL as your LLM provider base_url; AgentSentry will then
    intercept every call. Use `policy_check()` for out-of-band evaluation
    (e.g. inside a custom tool wrapper).
    """
    gateway_url: str
    api_key:     str
    agent_id:    str
    control_url: str | None = None
    timeout:     float      = 5.0

    def headers(self) -> dict[str, str]:
        return {
            "x-agentsentry-key":   self.api_key,
            "x-agentsentry-agent": self.agent_id,
        }

    def policy_check(self, package: str, input_: dict[str, Any]) -> PolicyDecision:
        url = f"{self.gateway_url.rstrip('/')}/v1/policy/check"
        try:
            r = httpx.post(url, json={"package": package, "input": input_},
                           headers=self.headers(), timeout=self.timeout)
            r.raise_for_status()
            return PolicyDecision.from_dict(r.json())
        except Exception:
            # Fail-open at the SDK level; the gateway is still the enforcement point.
            return PolicyDecision()
