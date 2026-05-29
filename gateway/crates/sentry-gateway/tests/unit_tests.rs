//! Unit tests for redaction, policy evaluation, and SSE usage parsing.

use serde_json::json;

use sentry_gateway::policy::{PolicyDef, PolicyStore};
use sentry_gateway::redact;
use sentry_gateway::span::hash_api_key;

#[test]
fn redact_email_and_secret() {
    let s = "Contact alice@example.com using sk-ABCDEFGHIJKLMNOPQRSTUVWX1234567890ABCD now";
    let (out, hits) = redact::scan_and_redact(s, true, true);
    assert!(!out.contains("alice@example.com"));
    assert!(!out.contains("sk-ABCDEFGHIJKLMNOPQRSTUVWX1234567890ABCD"));
    assert!(hits.iter().any(|h| h.kind == "email"));
    assert!(hits.iter().any(|h| h.kind == "openai_key"));
}

#[test]
fn redact_disabled_passes_through() {
    let s = "alice@example.com";
    let (out, hits) = redact::scan_and_redact(s, false, false);
    assert_eq!(out, s);
    assert!(hits.is_empty());
}

#[test]
fn policy_default_allow_when_empty() {
    let store = PolicyStore::new();
    let d = store.evaluate("tool_call", &json!({}));
    assert!(d.allow);
}

#[test]
fn policy_deny_external_email() {
    let rego = r#"
        package agentsentry.tool_call
        import future.keywords.if
        default decision := {"allow": true}
        decision := {
            "allow": false,
            "reason": "external email recipient",
            "policy_id": "pol_block_external_email"
        } if {
            input.tool.name == "send_email"
            not endswith(input.tool.args.to, "@contoso.com")
        }
    "#;
    let store = PolicyStore::new();
    store.replace_all(vec![PolicyDef {
        id: "pol_block_external_email".into(),
        name: "block external".into(),
        language: "rego".into(),
        source: rego.into(),
        status: "enforced".into(),
    }]);
    let allow = store.evaluate("tool_call", &json!({
        "tool": {"name":"send_email","args":{"to":"bob@contoso.com"}}
    }));
    assert!(allow.allow, "internal recipient must be allowed");

    let deny = store.evaluate("tool_call", &json!({
        "tool": {"name":"send_email","args":{"to":"bob@gmail.com"}}
    }));
    assert!(!deny.allow, "external recipient must be denied");
    assert_eq!(deny.policy_id, "pol_block_external_email");
}

#[test]
fn policy_draft_is_ignored() {
    let rego = r#"
        package agentsentry.tool_call
        import future.keywords.if
        default decision := {"allow": true}
        decision := {"allow": false, "reason": "drafted", "policy_id": "p"} if true
    "#;
    let store = PolicyStore::new();
    store.replace_all(vec![PolicyDef {
        id: "p".into(), name: "n".into(), language: "rego".into(),
        source: rego.into(), status: "draft".into(),
    }]);
    let d = store.evaluate("tool_call", &json!({}));
    assert!(d.allow);
}

#[test]
fn hash_api_key_is_deterministic_and_empty_safe() {
    assert_eq!(hash_api_key(""), "");
    let a = hash_api_key("sk_test");
    let b = hash_api_key("sk_test");
    assert_eq!(a, b);
    assert_eq!(a.len(), 64); // hex sha256
}
