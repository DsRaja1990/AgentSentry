//! AgentSentry Gateway — runtime data-plane proxy.
//!
//! Intercepts MCP and LLM provider traffic, evaluates policy in-process via
//! regorus (Rego), redacts PII / secrets, and emits OTel-GenAI-shaped spans
//! to the AgentSentry control plane.

pub mod config;
pub mod identity;
pub mod policy;
pub mod redact;
pub mod span;
pub mod telemetry;
pub mod proxy;
pub mod state;
