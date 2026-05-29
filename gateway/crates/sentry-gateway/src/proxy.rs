use crate::identity::WorkloadIdentity;
use crate::policy::PolicyDecision;
use crate::redact;
use crate::span::{hash_api_key, Span};
use crate::state::AppState;
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Instant;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz",              get(healthz))
        .route("/v1/policy/check",      post(policy_check))
        .route("/v1/openai/*rest",      any(proxy_openai))
        .route("/v1/azure/*rest",       any(proxy_azure))
        .route("/v1/anthropic/*rest",   any(proxy_anthropic))
        .route("/v1/gemini/*rest",      any(proxy_gemini))
        .route("/v1/mcp",               post(proxy_mcp))
        .with_state(state)
}

async fn healthz() -> &'static str { "ok" }

#[derive(Deserialize)]
struct PolicyCheckBody {
    package: String,
    input:   Value,
}

async fn policy_check(
    State(s): State<AppState>,
    Json(body): Json<PolicyCheckBody>,
) -> Json<PolicyDecision> {
    Json(s.policies.evaluate(&body.package, &body.input))
}

// --------------------------------------------------------------- providers
// Each provider has slightly different auth / usage shape. ProviderSpec
// captures the differences so proxy_llm stays generic.

#[derive(Clone, Copy, Debug)]
struct ProviderSpec {
    /// Provider key in `cfg.upstreams`.
    name: &'static str,
    /// Token field names in the JSON `usage` object.
    usage_in:  &'static str,
    usage_out: &'static str,
}

const OPENAI: ProviderSpec = ProviderSpec {
    name: "openai", usage_in: "prompt_tokens", usage_out: "completion_tokens",
};
const AZURE: ProviderSpec = ProviderSpec {
    // Same wire format as OpenAI, different base URL + `api-key` header.
    name: "azure", usage_in: "prompt_tokens", usage_out: "completion_tokens",
};
const ANTHROPIC: ProviderSpec = ProviderSpec {
    name: "anthropic", usage_in: "input_tokens", usage_out: "output_tokens",
};
const GEMINI: ProviderSpec = ProviderSpec {
    name: "gemini", usage_in: "promptTokenCount", usage_out: "candidatesTokenCount",
};

async fn proxy_openai(
    State(s): State<AppState>, Path(rest): Path<String>,
    method: Method, uri: Uri, headers: HeaderMap, body: Bytes,
) -> Response { proxy_llm(s, OPENAI, &rest, method, uri, headers, body).await }

async fn proxy_azure(
    State(s): State<AppState>, Path(rest): Path<String>,
    method: Method, uri: Uri, headers: HeaderMap, body: Bytes,
) -> Response { proxy_llm(s, AZURE, &rest, method, uri, headers, body).await }

async fn proxy_anthropic(
    State(s): State<AppState>, Path(rest): Path<String>,
    method: Method, uri: Uri, headers: HeaderMap, body: Bytes,
) -> Response { proxy_llm(s, ANTHROPIC, &rest, method, uri, headers, body).await }

async fn proxy_gemini(
    State(s): State<AppState>, Path(rest): Path<String>,
    method: Method, uri: Uri, headers: HeaderMap, body: Bytes,
) -> Response { proxy_llm(s, GEMINI, &rest, method, uri, headers, body).await }

// ------------------------------------------------------------- proxy core
async fn proxy_llm(
    state: AppState, provider: ProviderSpec, rest: &str,
    method: Method, uri: Uri, headers: HeaderMap, body: Bytes,
) -> Response {
    let start = Instant::now();
    let identity = WorkloadIdentity::from_headers(&headers);
    let mut span = Span::new_llm(&identity.agent_id, provider.name);
    span.caller_key_hash = hash_api_key(&identity.api_key);

    // 1. Redact inbound.
    let body_str = String::from_utf8_lossy(&body).to_string();
    let (redacted_in, hits_in) = redact::scan_and_redact(
        &body_str,
        state.cfg.redact.enable_pii,
        state.cfg.redact.enable_secrets,
    );
    for h in &hits_in { span.guardrail_hits.push(format!("in:{}", h.kind)); }

    // 2. Parse body for policy + stream detection.
    let parsed: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    if let Some(m) = parsed.get("model").and_then(|v| v.as_str()) {
        span.model = m.to_string();
    }
    let is_stream = parsed.get("stream").and_then(|v| v.as_bool()).unwrap_or(false)
        || rest.ends_with(":streamGenerateContent")    // Gemini
        || rest.contains("/stream");                    // Anthropic /v1/messages?stream

    let policy_input = json!({
        "agent":    { "id": identity.agent_id, "key_hash": span.caller_key_hash },
        "provider": provider.name,
        "request": {
            "path":  rest,
            "model": span.model,
            "body":  parsed,
            "streaming": is_stream,
        }
    });
    let decision = state.policies.evaluate("llm_call", &policy_input);
    span.policy_decision = if decision.allow { "allow".into() } else { "deny".into() };
    span.policy_id       = decision.policy_id.clone();
    span.policy_reason   = decision.reason.clone();
    span.name            = format!("{}.{}", provider.name, rest);
    span.streamed        = is_stream;

    if !decision.allow {
        span.duration_ms = start.elapsed().as_millis() as u64;
        span.http_status = 403;
        state.telemetry.enqueue(span);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": { "type": "agentsentry_policy_denied",
                           "reason": decision.reason,
                           "policy_id": decision.policy_id }
            })),
        ).into_response();
    }

    // 3. Build upstream URL.
    let upstream_base = match state.cfg.upstreams.get(provider.name) {
        Some(u) => u.base_url.clone(),
        None => return (StatusCode::BAD_GATEWAY,
                        format!("unknown provider: {}", provider.name)).into_response(),
    };
    let qs = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let url = format!("{}/{}{}", upstream_base.trim_end_matches('/'), rest, qs);

    // 4. Forward request (stripping our headers + accept-encoding so we can
    //    redact plaintext responses).
    let mut req = state.http.request(method.clone(), &url).body(redacted_in.into_bytes());
    for (name, value) in headers.iter() {
        let n = name.as_str().to_ascii_lowercase();
        if matches!(n.as_str(),
            "host" | "content-length" | "accept-encoding" |
            "x-agentsentry-key" | "x-agentsentry-agent" |
            "connection" | "transfer-encoding" | "te" | "upgrade" |
            "proxy-authenticate" | "proxy-authorization"
        ) { continue; }
        req = req.header(name, value);
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "upstream call failed");
            span.duration_ms = start.elapsed().as_millis() as u64;
            span.http_status = 502;
            state.telemetry.enqueue(span);
            return (StatusCode::BAD_GATEWAY, format!("upstream: {}", e)).into_response();
        }
    };
    let status     = resp.status();
    let resp_hdrs  = resp.headers().clone();
    span.http_status = status.as_u16();

    // Detect SSE from response headers OR client opt-in.
    let resp_is_sse = resp_hdrs.get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false);

    if is_stream || resp_is_sse {
        return stream_response(state.clone(), span, start, provider, status, resp_hdrs, resp);
    }

    // Non-stream: buffer + redact + parse usage.
    let resp_bytes = resp.bytes().await.unwrap_or_default();
    let resp_str   = String::from_utf8_lossy(&resp_bytes).to_string();
    let (redacted_out, hits_out) = redact::scan_and_redact(
        &resp_str,
        state.cfg.redact.enable_pii,
        state.cfg.redact.enable_secrets,
    );
    for h in &hits_out { span.guardrail_hits.push(format!("out:{}", h.kind)); }

    if let Ok(j) = serde_json::from_str::<Value>(&resp_str) {
        extract_usage(&j, provider, &mut span);
        if span.model.is_empty() {
            if let Some(m) = j.get("model").and_then(|v| v.as_str()) {
                span.model = m.to_string();
            }
        }
    }
    span.duration_ms = start.elapsed().as_millis() as u64;
    state.telemetry.enqueue(span);

    let mut out = Response::builder().status(status);
    for (n, v) in resp_hdrs.iter() {
        let ln = n.as_str().to_ascii_lowercase();
        if matches!(ln.as_str(), "transfer-encoding" | "content-length" | "connection") { continue; }
        out = out.header(n, v);
    }
    out.body(Body::from(redacted_out)).unwrap()
}

// ------------------------------------------------------------- streaming
fn stream_response(
    state:     AppState,
    mut span:  Span,
    start:     Instant,
    provider:  ProviderSpec,
    status:    StatusCode,
    resp_hdrs: reqwest::header::HeaderMap,
    resp:      reqwest::Response,
) -> Response {
    use tokio::sync::mpsc;
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);

    tokio::spawn(async move {
        let mut byte_stream = resp.bytes_stream();
        // Accumulate text only enough to scan SSE events for `usage` lines.
        let mut accumulated = String::new();
        let enable_pii     = state.cfg.redact.enable_pii;
        let enable_secrets = state.cfg.redact.enable_secrets;
        while let Some(item) = byte_stream.next().await {
            match item {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk).to_string();
                    let (red, hits) = redact::scan_and_redact(&text, enable_pii, enable_secrets);
                    for h in &hits { span.guardrail_hits.push(format!("stream:{}", h.kind)); }
                    accumulated.push_str(&red);
                    if tx.send(Ok(Bytes::from(red.into_bytes()))).await.is_err() {
                        break; // client gone
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "stream upstream error");
                    let _ = tx.send(Err(std::io::Error::new(
                        std::io::ErrorKind::Other, e))).await;
                    break;
                }
            }
        }
        extract_stream_usage(&accumulated, provider, &mut span);
        span.duration_ms = start.elapsed().as_millis() as u64;
        state.telemetry.enqueue(span);
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let body = Body::from_stream(stream);
    let mut out = Response::builder().status(status);
    for (n, v) in resp_hdrs.iter() {
        let ln = n.as_str().to_ascii_lowercase();
        if matches!(ln.as_str(), "transfer-encoding" | "content-length" | "connection") { continue; }
        out = out.header(n, v);
    }
    out.body(body).unwrap()
}

// ------------------------------------------------------------- usage parsing

fn extract_usage(j: &Value, provider: ProviderSpec, span: &mut Span) {
    // Common JSON shapes per provider:
    //   OpenAI/Azure:  { "usage": { "prompt_tokens", "completion_tokens" } }
    //   Anthropic:     { "usage": { "input_tokens",  "output_tokens"     } }
    //   Gemini:        { "usageMetadata": { "promptTokenCount", "candidatesTokenCount" } }
    let usage = match provider.name {
        "gemini" => j.get("usageMetadata"),
        _        => j.get("usage"),
    };
    if let Some(u) = usage {
        span.input_tokens  = u.get(provider.usage_in ).and_then(|v| v.as_u64()).unwrap_or(0);
        span.output_tokens = u.get(provider.usage_out).and_then(|v| v.as_u64()).unwrap_or(0);
    }
}

/// Scan an accumulated SSE stream for the *last* usage record.
/// - OpenAI emits `data: {... "usage": {...}}` only when
///   `stream_options.include_usage = true`.
/// - Anthropic emits `event: message_delta\n data: {... "usage": {...}}`.
/// - Gemini emits NDJSON-style chunks each carrying `usageMetadata`.
fn extract_stream_usage(accumulated: &str, provider: ProviderSpec, span: &mut Span) {
    let mut last: Option<Value> = None;
    for line in accumulated.lines() {
        let l = line.trim_start();
        let payload = if let Some(rest) = l.strip_prefix("data:") { rest.trim() }
                      else if l.starts_with('{')                  { l }
                      else                                         { continue };
        if payload == "[DONE]" { continue; }
        if let Ok(v) = serde_json::from_str::<Value>(payload) {
            let has_usage = match provider.name {
                "gemini" => v.get("usageMetadata").is_some(),
                _        => v.get("usage").is_some(),
            };
            if has_usage { last = Some(v); }
        }
    }
    if let Some(v) = last {
        extract_usage(&v, provider, span);
    }
}

// ------------------------------------------------------------------- MCP
async fn proxy_mcp(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let identity = WorkloadIdentity::from_headers(&headers);
    let mut span = Span::new_llm(&identity.agent_id, "mcp");
    span.caller_key_hash = hash_api_key(&identity.api_key);
    span.kind = "mcp".into();
    span.name = body.get("method").and_then(|v| v.as_str()).unwrap_or("mcp.call").to_string();

    let (tool_name, tool_args) = match span.name.as_str() {
        "tools/call" => {
            let params = body.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            (name, args)
        }
        _ => (String::new(), Value::Null),
    };
    span.tool_name = tool_name.clone();

    let policy_input = json!({
        "agent":    { "id": identity.agent_id, "key_hash": span.caller_key_hash },
        "provider": "mcp",
        "tool":     { "name": tool_name, "args": tool_args },
        "raw":      body,
    });
    let decision = s.policies.evaluate("tool_call", &policy_input);
    span.policy_decision = if decision.allow { "allow".into() } else { "deny".into() };
    span.policy_id       = decision.policy_id.clone();
    span.policy_reason   = decision.reason.clone();

    if !decision.allow {
        span.http_status = 403;
        s.telemetry.enqueue(span);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "jsonrpc": "2.0",
                "error": { "code": -32001,
                           "message": "agentsentry_policy_denied",
                           "data": { "reason": decision.reason,
                                     "policy_id": decision.policy_id } }
            })),
        ).into_response();
    }

    span.http_status = 200;
    s.telemetry.enqueue(span);
    Json(json!({
        "jsonrpc": "2.0",
        "result":  { "agentsentry": "approved", "policy_id": decision.policy_id }
    })).into_response()
}
