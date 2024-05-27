// src/main.rs

mod dataset_report;
mod archive_report;
mod utils;

use std::env;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // Initialize the logger
    init_logger();

    info!("Starting report generation...");

    if let Err(e) = run_reports().await {
        error!("Error running reports: {:?}", e);
    } else {
        info!("Report generation completed successfully.");
    }
}

async fn run_reports() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting Dataset Checker...");
    dataset_report::generate_dataset_report().await?;
    info!("Dataset Checker completed.");

    info!("Starting Archive Checker...");
    archive_report::generate_archive_report().await?;
    info!("Archive Checker completed.");

    Ok(())
}

fn init_logger() {
    // Initialize logging based on LOG_LEVEL environment variable
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_filter = match log_level.as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_filter)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
}
