use crate::policy::{PolicyDef, PolicyStore};
use crate::span::Span;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

const CHANNEL_CAPACITY: usize = 1024;
const BATCH_SIZE: usize       = 64;
const FLUSH_MS: u64           = 250;

/// Front-end handle: clone-able, used by request handlers to enqueue spans
/// without awaiting network I/O. Drops with a warning if the worker is
/// overloaded — that is intentional backpressure.
#[derive(Clone)]
pub struct Telemetry {
    tx:          mpsc::Sender<Span>,
    client:      reqwest::Client,
    control_url: String,
    api_key:     String,
}

#[derive(Serialize)]
struct IngestBody<'a> { spans: &'a [Span] }

#[derive(Deserialize)]
struct PolicyBundle { #[serde(default)] policies: Vec<PolicyDef> }

impl Telemetry {
    pub fn start(control_url: String, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client");
        let (tx, rx) = mpsc::channel::<Span>(CHANNEL_CAPACITY);

        let worker = Worker {
            rx,
            client: client.clone(),
            control_url: control_url.clone(),
            api_key: api_key.clone(),
        };
        tokio::spawn(worker.run());

        Self { tx, client, control_url, api_key }
    }

    /// Enqueue a span; non-blocking. Drops with a warning if the channel
    /// is full so the request hot path never stalls.
    pub fn enqueue(&self, span: Span) {
        if let Err(e) = self.tx.try_send(span) {
            match e {
                mpsc::error::TrySendError::Full(_) =>
                    tracing::warn!("telemetry channel full — dropping span"),
                mpsc::error::TrySendError::Closed(_) =>
                    tracing::error!("telemetry channel closed"),
            }
        }
    }

    pub async fn pull_policies(&self, store: &PolicyStore) {
        let url = format!("{}/v1/policies/bundle", self.control_url.trim_end_matches('/'));
        match self.client.get(&url).bearer_auth(&self.api_key).send().await {
            Ok(r) if r.status().is_success() => {
                match r.json::<PolicyBundle>().await {
                    Ok(b) => {
                        let n = b.policies.len();
                        store.replace_all(b.policies);
                        tracing::info!(count = n, "policy bundle refreshed");
                    }
                    Err(e) => tracing::warn!(error = %e, "policy bundle decode"),
                }
            }
            Ok(r) => tracing::warn!(status = %r.status(), "policy bundle non-2xx"),
            Err(e) => tracing::warn!(error = %e, "policy bundle pull failed"),
        }
    }

    pub async fn run_policy_poller(self, store: PolicyStore, interval: Duration) {
        let mut t = tokio::time::interval(interval);
        loop {
            t.tick().await;
            self.pull_policies(&store).await;
        }
    }
}

struct Worker {
    rx:          mpsc::Receiver<Span>,
    client:      reqwest::Client,
    control_url: String,
    api_key:     String,
}

impl Worker {
    async fn run(mut self) {
        let mut buf: Vec<Span> = Vec::with_capacity(BATCH_SIZE);
        let flush = Duration::from_millis(FLUSH_MS);
        loop {
            match self.rx.recv().await {
                None => { self.flush(&mut buf).await; return; }
                Some(s) => buf.push(s),
            }
            // Opportunistically drain up to BATCH_SIZE without blocking.
            while buf.len() < BATCH_SIZE {
                match self.rx.try_recv() {
                    Ok(s) => buf.push(s),
                    Err(_) => break,
                }
            }
            if buf.len() < BATCH_SIZE {
                tokio::time::sleep(flush).await;
            }
            self.flush(&mut buf).await;
        }
    }

    async fn flush(&self, buf: &mut Vec<Span>) {
        if buf.is_empty() { return; }
        let url = format!("{}/v1/ingest", self.control_url.trim_end_matches('/'));
        let body = IngestBody { spans: buf.as_slice() };
        for attempt in 0..3u32 {
            let res = self.client.post(&url).bearer_auth(&self.api_key)
                .json(&body).send().await;
            match res {
                Ok(r) if r.status().is_success() => {
                    tracing::debug!(count = buf.len(), "telemetry flushed");
                    buf.clear();
                    return;
                }
                Ok(r) => tracing::warn!(status = %r.status(), attempt, "ingest non-2xx"),
                Err(e) => tracing::warn!(error = %e, attempt, "ingest failed"),
            }
            tokio::time::sleep(Duration::from_millis(200 * (attempt as u64 + 1))).await;
        }
        tracing::error!(count = buf.len(), "dropping spans after 3 retries");
        buf.clear();
    }
}
