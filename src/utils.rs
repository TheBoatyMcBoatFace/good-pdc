// src/utils.rs

use reqwest::Client;
use tracing::{debug, error};
use sentry::add_breadcrumb;
use sentry::Breadcrumb;

pub async fn is_url_reachable(url: &str) -> bool {
    debug!("Checking URL: {}", url);

    add_breadcrumb(Breadcrumb {
        message: Some(format!("Checking URL: {}", url)),
        ..Default::default()
    });

    let client = Client::new();
    match client.get(url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                debug!("URL is reachable: {}", url);
                true
            } else {
                let warn_msg = format!("URL is not reachable (status: {}): {}", response.status(), url);
                error!("{}", warn_msg);
                sentry::capture_message(&warn_msg, sentry::Level::Warning);
                false
            }
        }
        Err(e) => {
            let err_msg = format!("Failed to reach URL (error: {:?}): {}", e, url);
            error!("{}", err_msg);
            sentry::capture_message(&err_msg, sentry::Level::Error);
            false
        }
    }
}
