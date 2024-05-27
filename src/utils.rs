// src/utils.rs
use reqwest::blocking::Client;

pub fn is_url_reachable(url: &str) -> bool {
    let client = Client::new();
    let response = client.get(url).send();
    response.is_ok() && response.unwrap().status().is_success()
}
