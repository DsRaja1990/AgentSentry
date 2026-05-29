use axum::http::HeaderMap;

/// Workload identity resolved from an inbound request.
/// MVP: bearer API key carried via `Authorization: Bearer <key>` or
///       `x-agentsentry-key` header. Phase 2 replaces with SPIFFE SVID.
#[derive(Debug, Clone)]
pub struct WorkloadIdentity {
    pub api_key:    String,
    pub agent_id:   String,
    pub tenant_id:  String,
    pub project_id: String,
}

impl WorkloadIdentity {
    pub fn anonymous() -> Self {
        Self {
            api_key: String::new(),
            agent_id: "unknown".into(),
            tenant_id: "t_default".into(),
            project_id: "p_default".into(),
        }
    }

    pub fn from_headers(headers: &HeaderMap) -> Self {
        let api_key = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").or_else(|| s.strip_prefix("bearer ")))
            .map(|s| s.to_string())
            .or_else(|| {
                headers.get("x-agentsentry-key")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        let agent_id = headers.get("x-agentsentry-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        Self {
            api_key,
            agent_id,
            tenant_id: "t_default".into(),
            project_id: "p_default".into(),
        }
    }
}
