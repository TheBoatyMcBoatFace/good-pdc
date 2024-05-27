// src/utils.rs

use reqwest::Client;

pub async fn is_url_reachable(url: &str) -> bool {
    let client = Client::new();
    match client.get(url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}
