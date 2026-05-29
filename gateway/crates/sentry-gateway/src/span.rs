use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id:  String,
    pub span_id:   String,
    pub parent_id: String,

    pub tenant_id:  String,
    pub project_id: String,
    pub agent_id:   String,

    pub ts:          String,
    pub duration_ms: u64,

    pub kind:     String,
    pub name:     String,
    pub model:    String,
    pub provider: String,

    pub input_tokens:  u64,
    pub output_tokens: u64,
    pub cost_usd:      f64,

    pub tool_name:            String,
    pub tool_args_redacted:   String,
    pub tool_result_redacted: String,

    pub policy_decision: String,
    pub policy_id:       String,
    pub policy_reason:   String,

    pub guardrail_hits: Vec<String>,
    pub attributes:     HashMap<String, String>,

    /// Hex-encoded SHA-256 of the inbound caller's API key. The control plane
    /// resolves this to a tenant during ingest so the gateway never needs to
    /// know per-tenant routing rules. Empty means "use auth context tenant".
    #[serde(default)]
    pub caller_key_hash: String,

    /// `true` if the upstream response was streamed (SSE). Token counts may
    /// be incomplete when the stream did not include a usage record.
    #[serde(default)]
    pub streamed: bool,

    /// HTTP status returned to the caller.
    #[serde(default)]
    pub http_status: u16,
}

impl Span {
    pub fn new_llm(agent_id: &str, provider: &str) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let mut attrs = HashMap::new();
        attrs.insert("agentsentry.component".into(), "gateway".into());
        Self {
            trace_id:  Uuid::new_v4().simple().to_string(),
            span_id:   Uuid::new_v4().simple().to_string()[..16].into(),
            parent_id: String::new(),
            tenant_id:  "t_default".into(),
            project_id: "p_default".into(),
            agent_id:   agent_id.into(),
            ts:          now,
            duration_ms: 0,
            kind:     "llm".into(),
            name:     String::new(),
            model:    String::new(),
            provider: provider.into(),
            input_tokens:  0,
            output_tokens: 0,
            cost_usd:      0.0,
            tool_name:            String::new(),
            tool_args_redacted:   String::new(),
            tool_result_redacted: String::new(),
            policy_decision: "allow".into(),
            policy_id:       String::new(),
            policy_reason:   String::new(),
            guardrail_hits:  vec![],
            attributes:      attrs,
            caller_key_hash: String::new(),
            streamed: false,
            http_status: 0,
        }
    }
}

/// Hex-encoded SHA-256 of a raw API key. Empty input returns an empty string.
pub fn hash_api_key(raw: &str) -> String {
    if raw.is_empty() { return String::new(); }
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    hex::encode(h.finalize())
}
