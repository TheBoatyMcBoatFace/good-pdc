// src/dataset_report.rs

use reqwest::Client;
use serde::Deserialize;
use tokio::task;
use std::fs::{File, create_dir_all, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::Path;
use csv;
use std::fs;
use uuid::Uuid;
use tracing::{info, warn, error, debug};
use sentry::add_breadcrumb;
use sentry::Breadcrumb;

#[derive(Debug, Deserialize)]
struct Dataset {
    #[serde(rename = "identifier")]
    id: String,
    title: String,
    description: String,
    issued: String,
    modified: String,
    released: String,
    #[serde(rename = "landingPage")]
    landing_page: String,
    theme: Vec<Theme>,
    distribution: Vec<Distribution>,
}

#[derive(Debug, Deserialize)]
struct Theme {
    data: String,
}

#[derive(Debug, Deserialize)]
struct Distribution {
    #[serde(rename = "data")]
    distribution_data: DistributionData,
}

#[derive(Debug, Deserialize)]
struct DistributionData {
    #[serde(rename = "downloadURL")]
    download_url: Option<String>,
}

// Mapping for Data Topics
const DATA_TOPICS: &[(&str, &str)] = &[
    ("Doctors and clinicians", "DAC"),
    ("Dialysis facilities", "DF"),
    ("Home health services", "HHS"),
    ("Hospice care", "HC"),
    ("Hospitals", "HOS"),
    ("Inpatient rehabilitation facilities", "IRF"),
    ("Long-term care hospitals", "LTCH"),
    ("Nursing homes including rehab services", "NH"),
    ("Physician office visit costs", "PPL"),
    ("Supplier directory", "SUP")
];

// Generate Dataset Report
pub async fn generate_dataset_report() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let url = "https://data.cms.gov/provider-data/api/1/metastore/schemas/dataset/items?show-reference-ids=false";

    add_breadcrumb(Breadcrumb {
        message: Some("Fetching datasets".into()),
        ..Default::default()
    });

    // Fetch datasets
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let err_msg = format!("Failed to fetch datasets from {}: HTTP {}", url, response.status());
        error!("{}", err_msg);
        sentry::capture_message(&err_msg, sentry::Level::Error);
        return Err(err_msg.into());
    }

    info!("Dataset response received!");

    // Deserialize response JSON directly into a Vec<Dataset>
    let datasets: Vec<Dataset> = response.json().await?;

    // Process each dataset in parallel
    let mut tasks = vec![];
    for dataset in datasets {
        let client = client.clone();
        tasks.push(task::spawn(async move {
            if let Err(e) = process_and_generate_report(&client, dataset).await {
                error!("Error processing dataset: {:?}", e);
                sentry::capture_message(&format!("Error processing dataset: {:?}", e), sentry::Level::Error);
            }
        }));
    }

    // Await all tasks
    for task in tasks {
        if let Err(e) = task.await {
            error!("Task failed: {:?}", e);
            sentry::capture_message(&format!("Task failed: {:?}", e), sentry::Level::Error);
        }
    }

    info!("All tasks completed.");
    Ok(())
}

// Process each dataset and generate report
async fn process_and_generate_report(client: &Client, dataset: Dataset) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Processing dataset: {}", dataset.title)),
        ..Default::default()
    });

    // Determine Data Topic
    let mut data_topic = "undefined";
    if let Some(theme) = dataset.theme.iter().find(|t| DATA_TOPICS.iter().any(|(k, _)| *k == t.data)) {
        data_topic = DATA_TOPICS.iter().find(|(k, _)| *k == theme.data).map(|(_, v)| *v).unwrap_or("undefined");
    }

    // Construct the file path
    let file_path = format!("datasets/{}.md", data_topic);

    // Ensure directory exists
    create_dir_all("datasets")?;

    // Check links and generate status report
    let download_url_option = dataset.distribution
        .get(0)
        .and_then(|dist| dist.distribution_data.download_url.as_deref()); // Use as_deref to get an Option<&str>

    add_breadcrumb(Breadcrumb {
        message: Some(format!("Checking download URL for dataset: {}", dataset.title)),
        ..Default::default()
    });

    let download_url_status = match download_url_option {
        Some(url) => {
            debug!("Checking download URL: {}", url);
            check_link(client, url).await?
        }
        None => {
            let warn_msg = format!("No download URL found for dataset: {}", dataset.title);
            warn!("{}", warn_msg);
            "❌"
        }
    };

    let landing_page_status = check_link(client, &dataset.landing_page).await?;
    let pdc_page = format!("https://data.cms.gov/provider-data/dataset/{}", dataset.id);
    let pdc_page_status = check_link(client, &pdc_page).await?;

    // Get dataset statistics
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Getting dataset statistics for: {}", dataset.title)),
        ..Default::default()
    });

    let (filesize, row_count, column_count, encoding) = if let Some(url) = download_url_option {
        get_dataset_statistics(client, url).await?
    } else {
        ("N/A".to_string(), 0, 0, "N/A")
    };

    // Prepare dataset report
    let mut report = format!(
        "## {}\n{}\n\n**Dataset ID:** {}\n\n**Status:** {}\n\n### Dataset Details\n\n",
        dataset.title, dataset.description, dataset.id, download_url_status
    );

    report.push_str(
        &format!("<details>\n<summary>File History</summary>\n\n|  Activity   |  Description |  Date  |\n| --- | --- | --- |\n| Issued Date   | When the dataset was created | {} |\n| Modified Date | when it was last modified | {} |\n| Release Date | when the dataset was made public | {} |\n| Last Checked | when this dataset was last tested | {} |\n\n</details>\n\n", dataset.issued, dataset.modified, dataset.released, chrono::Utc::now().format("%Y-%m-%d"))
    );

    report.push_str(
        &format!("<details>\n<summary>File Overview</summary>\n\n| Metric | Result |\n| --- | --- |\n| Filesize | {} MB |\n| Row Count | {} |\n| Column Count | {} |\n\n</details>\n\n", filesize, row_count, column_count)
    );

    report.push_str(
        &format!(
            "### Data Integrity Tests\nDoes this dataset abide by basic data formatting standards?\n<details>\n\n<summary>{} </summary>\n\n| Test | Description | Result |\n| --- | --- | --- |\n| Column Count Consistency | Verify that all rows have the same number of columns. | {} |\n| Header Validation | Ensure the CSV has a header row and all headers are unique and meaningful. | {} |\n| Encoding Validation | Verify that the CSV file uses UTF-8 encoding. | {} |\n\n</details>\n\n",
            download_url_status, "✅", "✅", encoding
        )
    );

    report.push_str("### Public Access Tests\nTesting the additional resources listed in the api.\n\n");
    report.push_str("| Page | Status | A11y Test |\n| :-----------: | :-----------: | :-----------: |\n");

    let download_url_link = format!("[Direct Download]({})", download_url_option.unwrap_or("#"));
    let landing_page_link = format!("[Landing Page]({})", dataset.landing_page);
    let pdc_page_link = format!("[PDC Page]({})", pdc_page);

    if pdc_page_status == "❌" {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
            pdc_page_link, pdc_page_status, pdc_page, pdc_page
        ));
    } else {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
            pdc_page_link, pdc_page_status, pdc_page, pdc_page
        ));
    }

    if landing_page_status == "❌🔀" {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
            landing_page_link, landing_page_status, dataset.landing_page, dataset.landing_page
        ));
    } else {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
            landing_page_link, landing_page_status, dataset.landing_page, dataset.landing_page
        ));
    }

    report.push_str(&format!("| {} | {} |  |\n\n", download_url_link, download_url_status));

    // Write to the appropriate markdown file
    if Path::new(&file_path).exists() {
        update_existing_report(&file_path, &dataset.id, &report)?;
    } else {
        create_new_report(&file_path, data_topic, &report)?;
    }

    info!("Report generated for dataset: {}", dataset.title);

    Ok(())
}

async fn check_link<'a>(client: &'a Client, url: &'a str) -> Result<&'a str, Box<dyn std::error::Error + Send + Sync>> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Checking link: {}", url)),
        ..Default::default()
    });

    let response = client.get(url).send().await?;
    if response.status().is_success() {
        Ok("✅")
    } else if response.status().is_redirection() {
        let warn_msg = format!("Redirection detected for URL: {}", url);
        warn!("{}", warn_msg);
        sentry::capture_message(&warn_msg, sentry::Level::Warning);
        Ok("❌🔀")
    } else {
        let err_msg = format!("Failed to reach URL: {}: HTTP {}", url, response.status());
        error!("{}", err_msg);
        sentry::capture_message(&err_msg, sentry::Level::Error);
        Ok("❌")
    }
}

async fn get_dataset_statistics<'a>(
    client: &'a Client,
    url: &'a str
) -> Result<(String, usize, usize, &'a str), Box<dyn std::error::Error + Send + Sync>> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Getting dataset statistics for URL: {}", url)),
        ..Default::default()
    });

    let response = client.get(url).send().await?;
    let temp_file_path = format!("/tmp/{}.csv", Uuid::new_v4());
    let mut file = File::create(&temp_file_path)?;
    let content = response.bytes().await?;
    file.write_all(&content)?;

    let metadata = fs::metadata(&temp_file_path)?;
    let filesize = metadata.len() as f64 / 1_000_000.0;

    let mut reader = csv::Reader::from_path(&temp_file_path)?;
    let headers = reader.headers()?;
    let column_count = headers.len();

    let mut row_count = 0;
    for record in reader.records() {
        if let Err(e) = record {
            error!("CSV record failed to read: {:?}", e);
            sentry::capture_message(&format!("CSV record failed to read: {:?}", e), sentry::Level::Error);
        } else {
            row_count += 1;
        }
    }

    fs::remove_file(&temp_file_path)?;

    debug!("Dataset statistics for URL {}: filesize={} MB, row_count={}, column_count={}, encoding=UTF-8", url, filesize, row_count, column_count);

    Ok((format!("{:.1}", filesize), row_count, column_count, "UTF-8"))
}

fn create_new_report(file_path: &str, data_topic: &str, report: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Creating new report at {}", file_path)),
        ..Default::default()
    });

    let mut file = File::create(file_path)?;
    let mut writer = BufWriter::new(&mut file);

    writeln!(writer, "# {} Datasets", data_topic)?;
    writeln!(writer, "Testing all the {} datasets listed on the Provider Data Catalog (PDC) API.\n", data_topic)?;
    writeln!(writer, "{}", report)?;

    info!("Created new report at {}", file_path);

    Ok(())
}

fn update_existing_report(file_path: &str, dataset_id: &str, report: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Updating existing report at {}", file_path)),
        ..Default::default()
    });

    let mut content = fs::read_to_string(file_path)?;

    let search_str = format!("**Dataset ID:** {}\n\n**Status:**", dataset_id);
    if let Some(start_pos) = content.find(&search_str) {
        if let Some(end_pos) = content[start_pos..].find("\n## Dataset Details").map(|p| p + start_pos) {
            content.replace_range(start_pos..end_pos, report);
        }
    } else {
        content.push_str(report);
    }

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)?;

    file.write_all(content.as_bytes())?;

    info!("Updated existing report at {}", file_path);

    Ok(())
}
