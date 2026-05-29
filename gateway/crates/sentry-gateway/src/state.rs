use crate::config::Config;
use crate::policy::PolicyStore;
use crate::telemetry::Telemetry;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub cfg:        Arc<Config>,
    pub policies:   PolicyStore,
    pub telemetry:  Telemetry,
    pub http:       Client,
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        let telemetry = Telemetry::start(cfg.control_plane_url.clone(), cfg.api_key.clone());
        let policies  = PolicyStore::new();
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("upstream client");
        Self { cfg: Arc::new(cfg), policies, telemetry, http }
    }
}
