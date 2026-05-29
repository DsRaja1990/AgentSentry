# Integration: LangChain (Python) end-to-end

A complete demo: a LangChain `AgentExecutor` whose LLM calls flow through the
AgentSentry gateway and whose `send_email` tool is gated by a Rego policy.

## Run

```powershell
# 1. Boot the stack (from repo root)
docker compose -f deploy/docker-compose.dev.yml up -d

# 2. Install deps
cd integrations/langchain-python
pip install -r requirements.txt

# 3. Set keys
$env:OPENAI_API_KEY = "sk-..."
$env:SENTRY_GATEWAY = "http://localhost:8080"
$env:SENTRY_API_KEY = "sk_dev_local_demo_key"

# 4. Run
python example.py
```

## What you should see

- Test 1 sends to `alice@contoso.com` — the tool runs and returns "[sent]".
- Test 2 sends to `customer@gmail.com` — the policy `pol_block_external_email`
  denies it; the tool returns "[BLOCKED by AgentSentry: external email
  recipient (pol_block_external_email)]".
- Open <http://localhost:3000/traces> — both LLM calls and the policy
  decision appear, with badges (allow / deny).

## Files

- [example.py](example.py)         — the LangChain agent.
- [policy.rego](policy.rego)       — the seeded policy (for reference).
- [requirements.txt](requirements.txt).
