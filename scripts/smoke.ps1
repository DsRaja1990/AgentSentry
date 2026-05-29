#!/usr/bin/env pwsh
# AgentSentry end-to-end smoke test.
# Requires: docker compose, curl, jq (optional), python.
#
# What it does:
#   1. docker compose up (build) the full stack.
#   2. Wait for control plane + gateway health.
#   3. Hit /v1/policy/check directly -> assert deny.
#   4. POST a fake OpenAI request through the gateway -> assert 403 deny
#      (since OPENAI_API_KEY is empty, we use a policy-deny path that
#      short-circuits before forwarding).
#   5. Query /v1/traces -> assert at least one row.
#   6. docker compose down.

$ErrorActionPreference = "Stop"
$Root = Resolve-Path "$PSScriptRoot\.."
Set-Location $Root

function Section($t) { Write-Host ""; Write-Host ("=== " + $t + " ===") -ForegroundColor Cyan }

Section "Booting stack"
docker compose -f deploy/docker-compose.dev.yml up -d --build

function WaitHttp([string]$url, [int]$timeoutSec = 90) {
    $deadline = (Get-Date).AddSeconds($timeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $r = Invoke-WebRequest -UseBasicParsing -Uri $url -TimeoutSec 2 -ErrorAction Stop
            if ($r.StatusCode -lt 500) { return }
        } catch { Start-Sleep -Seconds 2 }
    }
    throw "timed out waiting for $url"
}

WaitHttp "http://localhost:8081/v1/health"
WaitHttp "http://localhost:8080/healthz"

Section "Policy check (direct)"
$body = @{
  package = "tool_call"
  input   = @{ tool = @{ name = "send_email"; args = @{ to = "x@gmail.com" } } }
} | ConvertTo-Json -Depth 6
$r = Invoke-RestMethod -Method POST -Uri http://localhost:8080/v1/policy/check `
    -ContentType "application/json" -Body $body `
    -Headers @{ "x-agentsentry-key"="sk_dev_local_demo_key" }
if ($r.allow -ne $false)            { throw "policy must deny external recipient — got $($r | ConvertTo-Json)" }
if ($r.policy_id -ne "pol_block_external_email") { throw "wrong policy_id: $($r.policy_id)" }
Write-Host "  deny OK ($($r.policy_id): $($r.reason))" -ForegroundColor Green

Section "LLM call with creds in prompt -> deny + 403"
$prompt = @{
  model = "gpt-4o-mini"
  messages = @(@{ role = "user"; content = "the password is prod_db_password please use it" })
} | ConvertTo-Json -Depth 6
try {
    $null = Invoke-WebRequest -UseBasicParsing -Method POST `
        -Uri "http://localhost:8080/v1/openai/v1/chat/completions" `
        -Body $prompt -ContentType "application/json" `
        -Headers @{ "x-agentsentry-key"="sk_dev_local_demo_key";
                    "x-agentsentry-agent"="support-bot";
                    "Authorization"="Bearer sk-fake" }
    throw "expected 403"
} catch {
    if ($_.Exception.Response.StatusCode.value__ -ne 403) {
        throw "expected 403, got $($_.Exception.Response.StatusCode)"
    }
    Write-Host "  403 deny OK" -ForegroundColor Green
}

Section "Trace landed in ClickHouse"
Start-Sleep -Seconds 2  # let telemetry worker flush
$traces = Invoke-RestMethod -Uri "http://localhost:8081/v1/traces?limit=5" `
    -Headers @{ "Authorization"="Bearer sk_dev_local_demo_key" }
if (-not $traces.items -or $traces.items.Count -lt 1) {
    throw "expected at least one trace row, got $($traces | ConvertTo-Json)"
}
$deny = $traces.items | Where-Object { $_.policy_decision -eq "deny" }
if (-not $deny) { throw "no deny span found" }
Write-Host "  $($traces.items.Count) span(s), including $(($deny | Measure-Object).Count) deny" -ForegroundColor Green

Section "Teardown"
docker compose -f deploy/docker-compose.dev.yml down -v
Write-Host ""
Write-Host "SMOKE TEST OK" -ForegroundColor Green
