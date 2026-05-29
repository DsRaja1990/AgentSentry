use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDef {
    pub id:       String,
    pub name:     String,
    pub language: String, // "rego" today
    pub source:   String,
    pub status:   String, // "enforced" | "monitor" | "draft"
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Redaction {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    #[serde(default)] pub require_approval: bool,
    #[serde(default)] pub redactions: Vec<Redaction>,
    #[serde(default)] pub reason: String,
    #[serde(default)] pub policy_id: String,
    #[serde(default)] pub obligations: Vec<String>,
}

impl PolicyDecision {
    pub fn allow_default() -> Self {
        Self {
            allow: true,
            require_approval: false,
            redactions: vec![],
            reason: String::new(),
            policy_id: "pol_default_allow".into(),
            obligations: vec![],
        }
    }
    pub fn deny(policy_id: &str, reason: &str) -> Self {
        Self {
            allow: false,
            require_approval: false,
            redactions: vec![],
            reason: reason.into(),
            policy_id: policy_id.into(),
            obligations: vec![],
        }
    }
}

/// Thread-safe policy store. The control plane pushes (or the gateway pulls)
/// `PolicyDef` lists; we cache them in memory and evaluate by cloning a
/// fresh regorus engine per call.
#[derive(Clone, Default)]
pub struct PolicyStore {
    inner: Arc<RwLock<Vec<PolicyDef>>>,
}

impl PolicyStore {
    pub fn new() -> Self { Self::default() }

    pub fn replace_all(&self, policies: Vec<PolicyDef>) {
        let mut w = self.inner.write();
        *w = policies;
    }

    pub fn snapshot(&self) -> Vec<PolicyDef> {
        self.inner.read().clone()
    }

    /// Evaluate `data.agentsentry.<package>.decision` against each enforced
    /// policy. First non-default deny wins; otherwise allow.
    pub fn evaluate(&self, package: &str, input: &serde_json::Value) -> PolicyDecision {
        let policies = self.snapshot();
        if policies.is_empty() {
            return PolicyDecision::allow_default();
        }
        let query = format!("data.agentsentry.{}.decision", package);

        for p in policies.iter().filter(|p| p.status == "enforced") {
            let mut engine = regorus::Engine::new();
            // Each policy must declare its own package; we add as a module.
            let module_name = format!("policy_{}.rego", p.id);
            if let Err(e) = engine.add_policy(module_name, p.source.clone()) {
                tracing::warn!(policy = %p.id, error = %e, "policy compile failed");
                continue;
            }
            let input_val = match regorus::Value::from_json_str(&input.to_string()) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, "policy input encode failed");
                    continue;
                }
            };
            engine.set_input(input_val);
            let results = match engine.eval_query(query.clone(), false) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(policy = %p.id, error = %e, "policy eval failed");
                    continue;
                }
            };
            if let Some(first) = results.result.into_iter().next() {
                if let Some(expr) = first.expressions.into_iter().next() {
                    let json = match expr.value.to_json_str() {
                        Ok(s) => s,
                        Err(e) => { tracing::warn!(error=%e, "decision json encode"); continue; }
                    };
                    if let Ok(mut decision) = serde_json::from_str::<PolicyDecision>(&json) {
                        if decision.policy_id.is_empty() {
                            decision.policy_id = p.id.clone();
                        }
                        if !decision.allow {
                            return decision;
                        }
                    }
                }
            }
        }
        PolicyDecision::allow_default()
    }
}
