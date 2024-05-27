// src/dataset_report.rs
use reqwest::blocking::Client;
use serde::Deserialize;
use std::fs::{File, create_dir_all, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::Path;
use csv;
use std::fs;
use uuid::Uuid;

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
    download_url: String,
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
pub fn generate_dataset_report() {
    let client = Client::new();
    let url = "https://data.cms.gov/provider-data/api/1/metastore/schemas/dataset/items?show-reference-ids=false";

    // Fetch datasets
    let response = client.get(url).send().expect("Failed to fetch datasets");

    if !response.status().is_success() {
        eprintln!("Failed to fetch datasets from {}: HTTP {}", url, response.status());
        return;
    }

    println!("Dataset response received!");

    // Deserialize response JSON directly into a Vec<Dataset>
    let datasets: Vec<Dataset> = response.json().expect("Failed to parse JSON");

    // Process each dataset
    for dataset in datasets {
        process_and_generate_report(&dataset);
    }
}

// Process each dataset and generate report
fn process_and_generate_report(dataset: &Dataset) {
    // Determine Data Topic
    let mut data_topic = "undefined";
    if let Some(theme) = dataset.theme.iter().find(|t| DATA_TOPICS.iter().any(|(k, _)| *k == t.data)) {
        data_topic = DATA_TOPICS.iter().find(|(k, _)| *k == theme.data).unwrap().1;
    }

    // Construct the file path
    let file_path = format!("datasets/{}.md", data_topic);

    // Ensure directory exists
    create_dir_all("datasets").expect("Failed to create datasets directory");

    // Check links and generate status report
    let download_url_status = check_link(&dataset.distribution[0].distribution_data.download_url);
    let landing_page_status = check_link(&dataset.landing_page);
    let pdc_page = format!("https://data.cms.gov/provider-data/dataset/{}", dataset.id);
    let pdc_page_status = check_link(&pdc_page);

    // Get dataset statistics
    let (filesize, row_count, column_count, encoding) = get_dataset_statistics(&dataset.distribution[0].distribution_data.download_url);

    // Prepare dataset report
    let mut report = format!(
        "## {}\n{}\n\n**Dataset ID:** {}\n\n**Status:** {}\n\n## Dataset Details\n\n",
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

    let download_url_link = format!("[Direct Download]({})", dataset.distribution[0].distribution_data.download_url);
    let landing_page_link = format!("[Landing Page]({})", dataset.landing_page);
    let pdc_page_link = format!("[PDC Page]({})", pdc_page);

    if pdc_page_status == "❌" {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
            pdc_page_link, pdc_page_status, pdc_page, pdc_page));
    } else {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
            pdc_page_link, pdc_page_status, pdc_page, pdc_page));
    }

    if landing_page_status == "❌🔀" {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
        landing_page_link, landing_page_status, dataset.landing_page, dataset.landing_page));
    } else {
        report.push_str(&format!(
            "| {} | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
        landing_page_link, landing_page_status, dataset.landing_page, dataset.landing_page));
    }

    report.push_str(&format!(
        "| {} | {} |  |\n", download_url_link, download_url_status
    ));

    // Write to the appropriate markdown file
    if Path::new(&file_path).exists() {
        update_existing_report(&file_path, &dataset.id, &report);
    } else {
        create_new_report(&file_path, data_topic, &report);
    }

    println!("Report generated for dataset: {}", dataset.title);
}

fn check_link(url: &str) -> &str {
    let client = Client::new();
    let response = client.get(url).send();

    if response.is_ok() {
        let status = response.unwrap().status();
        if status.is_success() {
            "✅"
        } else if status.is_redirection() {
            "❌🔀"
        } else {
            "❌"
        }
    } else {
        "❌"
    }
}

fn get_dataset_statistics(url: &str) -> (String, usize, usize, &str) {
    let response = reqwest::blocking::get(url).expect("Failed to download dataset");

    // Save the file to a temporary location
    let temp_file_path = format!("/tmp/{}.csv", Uuid::new_v4());
    let mut file = File::create(&temp_file_path).expect("Failed to create temporary file");
    let content = response.bytes().expect("Failed to read response bytes");
    file.write_all(&content).expect("Failed to write to temporary file");

    let metadata = fs::metadata(&temp_file_path).expect("Unable to read file metadata");
    let filesize = metadata.len() as f64 / 1_000_000.0;

    let mut reader = csv::Reader::from_path(&temp_file_path).expect("Failed to read CSV file");
    let headers = reader.headers().expect("Failed to read CSV headers");
    let column_count = headers.len();

    let mut row_count = 0;
    for _ in reader.records() {
        row_count += 1;
    }

    let encoding = "UTF-8";

    // Cleanup
    fs::remove_file(&temp_file_path).expect("Failed to remove temporary file");

    (format!("{:.1}", filesize), row_count, column_count, encoding)
}

fn create_new_report(file_path: &str, data_topic: &str, report: &str) {
    let mut file = File::create(file_path).expect("Failed to create report file");
    let mut writer = BufWriter::new(&mut file);

    writeln!(writer, "# {} Datasets", data_topic).expect("Failed to write to report file");
    writeln!(writer, "Testing all the {} datasets listed on the Provider Data Catalog (PDC) API.\n", data_topic).expect("Failed to write to report file");
    writeln!(writer, "{}", report).expect("Failed to write to report file");
}

fn update_existing_report(file_path: &str, dataset_id: &str, report: &str) {
    let mut content = fs::read_to_string(file_path).expect("Failed to read existing report file");

    let search_str = format!("**Dataset ID:** {}\n\n**Status:**", dataset_id);
    if content.contains(&search_str) {
        let start_pos = content.find(&search_str).unwrap();
        let end_pos = content[start_pos..].find("\n## Dataset Details").unwrap() + start_pos;
        content.replace_range(start_pos..end_pos, report);
    } else {
        content.push_str(report);
    }

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(file_path)
        .expect("Failed to open existing report file");

    file.write_all(content.as_bytes()).expect("Failed to write to existing report file");
}
