// src/utils.rs

use std::time::Duration;

use reqwest::Client;
use sentry::add_breadcrumb;
use sentry::Breadcrumb;
use tracing::{debug, error, warn};

const REQUEST_TIMEOUT_SECS: u64 = 30;
const CONNECT_TIMEOUT_SECS: u64 = 10;
const MAX_ATTEMPTS: u32 = 3;

/// The outcome of checking a single link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    Ok,
    Redirect,
    Broken,
}

impl LinkStatus {
    /// Emoji used in the generated Markdown reports.
    pub fn emoji(self) -> &'static str {
        match self {
            LinkStatus::Ok => "✅",
            LinkStatus::Redirect => "❌🔀",
            LinkStatus::Broken => "❌",
        }
    }

    pub fn is_ok(self) -> bool {
        matches!(self, LinkStatus::Ok)
    }
}

/// Build the single HTTP client shared across every check. Configured with
/// request/connect timeouts so one hanging URL can't stall the whole run.
pub fn build_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .user_agent(concat!(
            "good-pdc/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/TheBoatyMcBoatFace/good-pdc)"
        ))
        .build()
        .expect("failed to build HTTP client")
}

/// Number of datasets to process concurrently. Override with `MAX_CONCURRENCY`.
pub fn max_concurrency() -> usize {
    std::env::var("MAX_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(8)
}

/// Check a link, preferring `HEAD` (with a `GET` fallback for servers that
/// reject it) and retrying transient failures with exponential backoff.
pub async fn check_link(client: &Client, url: &str) -> LinkStatus {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Checking link: {}", url)),
        ..Default::default()
    });

    let mut last_err: Option<String> = None;

    for attempt in 0..MAX_ATTEMPTS {
        // Prefer HEAD; if the server rejects the method, fall back to GET.
        let response = match client.head(url).send().await {
            Ok(resp) if resp.status().is_client_error() => client.get(url).send().await,
            other => other,
        };

        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    debug!("URL is reachable: {}", url);
                    return LinkStatus::Ok;
                } else if status.is_redirection() {
                    warn!("Redirection detected for URL {}: HTTP {}", url, status);
                    return LinkStatus::Redirect;
                } else if status.is_server_error() {
                    // Transient server-side error: worth retrying.
                    last_err = Some(format!("HTTP {}", status));
                } else {
                    let msg = format!("URL is not reachable {}: HTTP {}", url, status);
                    warn!("{}", msg);
                    sentry::capture_message(&msg, sentry::Level::Warning);
                    return LinkStatus::Broken;
                }
            }
            Err(e) => {
                last_err = Some(format!("{:?}", e));
            }
        }

        if attempt + 1 < MAX_ATTEMPTS {
            let backoff = Duration::from_millis(250 * 2u64.pow(attempt));
            tokio::time::sleep(backoff).await;
        }
    }

    let msg = format!(
        "URL failed after {} attempts: {} ({})",
        MAX_ATTEMPTS,
        url,
        last_err.unwrap_or_default()
    );
    error!("{}", msg);
    sentry::capture_message(&msg, sentry::Level::Error);
    LinkStatus::Broken
}

/// Convenience wrapper returning a simple reachable/unreachable boolean.
pub async fn is_url_reachable(client: &Client, url: &str) -> bool {
    check_link(client, url).await.is_ok()
}
