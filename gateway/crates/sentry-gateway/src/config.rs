use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen_addr: String,

    pub control_plane_url: String,

    #[serde(default)]
    pub api_key: String,

    #[serde(default = "default_poll")]
    pub poll_interval_seconds: u64,

    #[serde(default)]
    pub redact: RedactConfig,

    #[serde(default)]
    pub upstreams: HashMap<String, Upstream>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedactConfig {
    #[serde(default = "default_true")]
    pub enable_pii: bool,
    #[serde(default = "default_true")]
    pub enable_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upstream {
    pub base_url: String,
}

fn default_listen() -> String { "0.0.0.0:8080".into() }
fn default_poll() -> u64 { 10 }
fn default_true() -> bool { true }

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        // env-var expansion: ${VAR} or ${VAR:-default}
        let expanded = expand_env(&text);
        let cfg: Config = serde_yaml::from_str(&expanded)?;
        Ok(cfg)
    }

    pub fn default_for_dev() -> Self {
        let mut upstreams = HashMap::new();
        upstreams.insert("openai".into(),    Upstream { base_url: "https://api.openai.com".into() });
        upstreams.insert("anthropic".into(), Upstream { base_url: "https://api.anthropic.com".into() });
        upstreams.insert("gemini".into(),    Upstream { base_url: "https://generativelanguage.googleapis.com".into() });
        // Azure base is per-resource; users must override via config or env.
        upstreams.insert("azure".into(),     Upstream { base_url: "https://YOUR-RESOURCE.openai.azure.com".into() });
        Self {
            listen_addr: default_listen(),
            control_plane_url: "http://localhost:8081".into(),
            api_key: String::new(),
            poll_interval_seconds: default_poll(),
            redact: RedactConfig::default(),
            upstreams,
        }
    }
}

fn expand_env(input: &str) -> String {
    let re = regex::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)(?::-(.*?))?\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let name = &caps[1];
        let default = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        std::env::var(name).unwrap_or_else(|_| default.to_string())
    })
    .into_owned()
}
