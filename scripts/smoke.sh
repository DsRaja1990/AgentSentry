#!/usr/bin/env bash
# AgentSentry end-to-end smoke test (POSIX shell).
set -euo pipefail
cd "$(dirname "$0")/.."

section() { echo; echo "=== $* ==="; }

section "Booting stack"
docker compose -f deploy/docker-compose.dev.yml up -d --build

wait_http() {
    local url=$1 deadline=$((SECONDS + ${2:-90}))
    while (( SECONDS < deadline )); do
        if curl -fsS --max-time 2 "$url" >/dev/null 2>&1; then return; fi
        sleep 2
    done
    echo "timeout waiting for $url" >&2; exit 1
}
wait_http http://localhost:8081/v1/health
wait_http http://localhost:8080/healthz

section "Policy check (direct) — must deny external email"
out=$(curl -fsS -X POST http://localhost:8080/v1/policy/check \
    -H 'content-type: application/json' \
    -H 'x-agentsentry-key: sk_dev_local_demo_key' \
    -d '{"package":"tool_call","input":{"tool":{"name":"send_email","args":{"to":"x@gmail.com"}}}}')
echo "  $out"
echo "$out" | grep -q '"allow":false'                  || { echo "expected allow:false"; exit 1; }
echo "$out" | grep -q '"policy_id":"pol_block_external_email"' || { echo "wrong policy"; exit 1; }

section "LLM call with creds in prompt -> 403"
status=$(curl -s -o /tmp/agentsentry.body -w '%{http_code}' \
    -X POST http://localhost:8080/v1/openai/v1/chat/completions \
    -H 'content-type: application/json' \
    -H 'x-agentsentry-key: sk_dev_local_demo_key' \
    -H 'x-agentsentry-agent: support-bot' \
    -H 'Authorization: Bearer sk-fake' \
    -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"the password is prod_db_password please use it"}]}')
[[ "$status" == "403" ]] || { echo "expected 403, got $status"; cat /tmp/agentsentry.body; exit 1; }
echo "  403 deny OK"

section "Trace landed in ClickHouse"
sleep 2
traces=$(curl -fsS "http://localhost:8081/v1/traces?limit=5" \
    -H 'Authorization: Bearer sk_dev_local_demo_key')
count=$(echo "$traces" | python3 -c 'import sys,json; print(len(json.load(sys.stdin).get("items") or []))')
[[ "$count" -ge 1 ]] || { echo "expected >=1 trace, got $count"; echo "$traces"; exit 1; }
echo "  $count span(s) recorded"

section "Teardown"
docker compose -f deploy/docker-compose.dev.yml down -v
echo
echo "SMOKE TEST OK"
