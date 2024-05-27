// src/main.rs

mod dataset_report;
mod archive_report;
mod utils;

#[tokio::main]
async fn main() {
    if let Err(e) = run_reports().await {
        eprintln!("Error running reports: {}", e);
    }
}

async fn run_reports() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Dataset Checker
    dataset_report::generate_dataset_report().await?;

    // Archive Checker
    archive_report::generate_archive_report().await?;

    Ok(())
}
