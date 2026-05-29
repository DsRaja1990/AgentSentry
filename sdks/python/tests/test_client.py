"""Unit tests for the AgentSentry Python SDK."""
from __future__ import annotations

import pytest

from agentsentry import PolicyDecision, Sentry


def test_headers_shape():
    s = Sentry(gateway_url="http://x", api_key="k", agent_id="a")
    h = s.headers()
    assert h["x-agentsentry-key"]   == "k"
    assert h["x-agentsentry-agent"] == "a"


def test_policy_decision_defaults_allow():
    d = PolicyDecision()
    assert d.allow is True
    assert d.redactions == []


def test_policy_decision_from_dict():
    d = PolicyDecision.from_dict({
        "allow": False, "reason": "no", "policy_id": "p1",
        "obligations": ["log"], "redactions": [{"path": "x", "kind": "email"}],
    })
    assert not d.allow
    assert d.reason == "no"
    assert d.policy_id == "p1"
    assert d.obligations == ["log"]
    assert d.redactions[0]["kind"] == "email"


def test_policy_check_fails_open(monkeypatch):
    s = Sentry(gateway_url="http://127.0.0.1:1", api_key="k", agent_id="a",
               timeout=0.1)
    d = s.policy_check("tool_call", {"x": 1})
    # No gateway reachable: SDK must fail-open with allow=True.
    assert d.allow is True


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-v"]))
