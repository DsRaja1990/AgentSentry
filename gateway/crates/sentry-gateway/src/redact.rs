use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionHit {
    pub kind:    String,
    pub matched: String,
}

static PATTERNS: Lazy<Vec<(&'static str, Regex)>> = Lazy::new(|| {
    vec![
        ("email",       Regex::new(r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b").unwrap()),
        ("credit_card", Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap()),
        ("ssn_us",      Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap()),
        ("aws_key",     Regex::new(r"AKIA[0-9A-Z]{16}").unwrap()),
        ("openai_key",  Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap()),
        ("github_pat",  Regex::new(r"gh[pousr]_[A-Za-z0-9]{20,}").unwrap()),
        ("jwt",         Regex::new(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").unwrap()),
    ]
});

/// Redact in place. Returns the list of hits and the redacted text.
pub fn scan_and_redact(text: &str, enable_pii: bool, enable_secrets: bool) -> (String, Vec<RedactionHit>) {
    let mut redacted = text.to_string();
    let mut hits = vec![];
    for (kind, re) in PATTERNS.iter() {
        let is_pii = matches!(*kind, "email" | "credit_card" | "ssn_us");
        if is_pii && !enable_pii        { continue; }
        if !is_pii && !enable_secrets   { continue; }
        for m in re.find_iter(text) {
            hits.push(RedactionHit { kind: (*kind).into(), matched: m.as_str().to_string() });
        }
        redacted = re.replace_all(&redacted, format!("«REDACTED:{}»", kind)).into_owned();
    }
    (redacted, hits)
}
