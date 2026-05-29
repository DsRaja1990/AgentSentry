use clap::Parser;
use sentry_gateway::{config::Config, proxy, state::AppState};
use std::{path::PathBuf, time::Duration};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "sentry-gateway", version, about = "AgentSentry runtime gateway")]
struct Args {
    /// Path to YAML config file (env vars expanded as ${VAR}).
    #[arg(long, env = "SENTRY_GW_CONFIG", default_value = "config.yaml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,sentry_gateway=info")))
        .with_target(false)
        .json()
        .init();

    let args = Args::parse();
    let cfg = if args.config.exists() {
        Config::load(&args.config)?
    } else {
        tracing::warn!(path = %args.config.display(),
            "config file not found, using dev defaults");
        Config::default_for_dev()
    };
    tracing::info!(listen = %cfg.listen_addr,
                   control = %cfg.control_plane_url, "gateway starting");

    let state = AppState::new(cfg.clone());
    let poll_interval = Duration::from_secs(cfg.poll_interval_seconds.max(1));

    // Initial pull + background poller.
    state.telemetry.pull_policies(&state.policies).await;
    {
        let t = state.telemetry.clone();
        let s = state.policies.clone();
        tokio::spawn(async move { t.run_policy_poller(s, poll_interval).await; });
    }

    let app = proxy::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
