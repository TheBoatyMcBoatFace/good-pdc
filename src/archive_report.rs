// src/archive_report.rs

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use crate::utils::is_url_reachable;
use futures::future::join_all;
use tracing::{info, warn, error, debug};

#[derive(Debug, Deserialize)]
struct Topics {
    #[serde(rename = "Dialysis facilities")]
    dialysis_facilities: Option<YearMap>,
    #[serde(rename = "Doctors and clinicians")]
    doctors_and_clinicians: Option<YearMap>,
    #[serde(rename = "Helpful Contacts")]
    helpful_contacts: Option<YearMap>,
    #[serde(rename = "Home health services")]
    home_health_services: Option<YearMap>,
    #[serde(rename = "Hospice care")]
    hospice_care: Option<YearMap>,
    #[serde(rename = "Hospitals")]
    hospitals: Option<YearMap>,
    #[serde(rename = "Inpatient rehabilitation facilities")]
    inpatient_rehabilitation_facilities: Option<YearMap>,
    #[serde(rename = "Long-term care hospitals")]
    long_term_care_hospitals: Option<YearMap>,
    #[serde(rename = "Nursing homes including rehab services")]
    nursing_homes_including_rehab_services: Option<YearMap>,
    #[serde(rename = "nh-backup")]
    nh_backup: Option<YearMap>,
    #[serde(rename = "Physician office visit costs")]
    physician_office_visit_costs: Option<YearMap>,
    #[serde(rename = "Supplier directory")]
    supplier_directory: Option<YearMap>,
}

#[derive(Debug, Deserialize)]
struct YearMap {
    #[serde(flatten)]
    years: HashMap<String, Vec<UrlEntry>>,
}

#[derive(Debug, Deserialize)]
struct UrlEntry {
    url: String,
    size: Option<u64>,
    #[serde(rename = "type")]
    entry_type: Option<String>,
    month: Option<String>,
    day: Option<String>,
}

impl UrlEntry {
    async fn formatted_entry(&self) -> String {
        let size_mb = self.size.map_or("N/A".to_string(), |s| format!("{:.1} MB", s as f64 / 1_000_000.0));
        let date = format!("{:02} / {:02} / 2020", self.month.as_ref().unwrap_or(&String::from("??")).parse::<u32>().unwrap_or(0), self.day.as_ref().unwrap_or(&String::from("??")).parse::<u32>().unwrap_or(0));
        let file_name = self.url.split('/').last().unwrap_or("Unknown file");
        let url = &self.url;
        let status = if is_url_reachable(url).await { "✅" } else { "❌" };

        debug!("Formatted entry: file_name={}, url={}, date={}, size_mb={}, status={}", file_name, url, date, size_mb, status);

        format!(
            "| [{}]({}) | {} | {} | {} |",
            file_name, self.url, date, size_mb, status
        )
    }

    async fn formatted_yearly_entry(&self) -> String {
        let size_mb = self.size.map_or("N/A".to_string(), |s| format!("{:.1} MB", s as f64 / 1_000_000.0));
        let url = &self.url;
        let status = if is_url_reachable(url).await { "✅" } else { "❌" };
        let emoji = if status == "✅" { "✅" } else { "❌" };

        debug!("Formatted yearly entry: url={}, size_mb={}, status={}", url, size_mb, status);

        format!(
            "{} **Yearly Archive** | [{}]({})\n  - **Size**: {}",
            emoji, self.url.split('/').last().unwrap_or("Unknown file"), self.url, size_mb
        )
    }
}

struct Summary {
    yearly_count: usize,
    monthly_count: usize,
    overall_status: bool,
}

impl Summary {
    fn new() -> Self {
        Self {
            yearly_count: 0,
            monthly_count: 0,
            overall_status: true,
        }
    }

    fn add_yearly(&mut self, status: bool) {
        self.yearly_count += 1;
        debug!("Added yearly entry: status={}", status);
        if !status {
            self.overall_status = false;
            warn!("Yearly entry failed");
        }
    }

    fn add_monthly(&mut self, status: bool) {
        self.monthly_count += 1;
        debug!("Added monthly entry: status={}", status);
        if !status {
            self.overall_status = false;
            warn!("Monthly entry failed");
        }
    }

    fn status_emoji(&self) -> &str {
        if self.overall_status { "✅" } else { "❌" }
    }
}

// Archive report function
pub async fn generate_archive_report() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = "https://data.cms.gov/provider-data/api/1/pdc/topics/archive";
    let client = Client::new();
    let response = match client.get(url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to send request to {}: {:?}", url, e);
            return Err(Box::new(e));
        }
    };

    if !response.status().is_success() {
        error!("Failed to fetch data from {}: HTTP {}", url, response.status());
        return Err(format!("Failed to fetch data from {}: HTTP {}", url, response.status()).into());
    }

    info!("Response received!");

    let topics: Topics = match response.json().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to parse JSON response: {:?}", e);
            return Err(Box::new(e));
        }
    };

    let mut output = String::new();
    let mut summary_output = String::new();
    output.push_str("# Archive Report\n\n");

    summary_output.push_str("| Data Topic | Yearly Archives | Monthly Archives | Status |\n");
    summary_output.push_str("|-----------|-----------|-----------|-----------|\n");

    let summary_dialysis = check_links_and_add_to_output("Dialysis facilities", topics.dialysis_facilities, &mut output).await;
    summary_output.push_str(&format!("| Dialysis facilities | {} | {} | {} |\n", summary_dialysis.yearly_count, summary_dialysis.monthly_count, summary_dialysis.status_emoji()));

    let summary_doctors = check_links_and_add_to_output("Doctors and clinicians", topics.doctors_and_clinicians, &mut output).await;
    summary_output.push_str(&format!("| Doctors and clinicians | {} | {} | {} |\n", summary_doctors.yearly_count, summary_doctors.monthly_count, summary_doctors.status_emoji()));

    let summary_helpful = check_links_and_add_to_output("Helpful Contacts", topics.helpful_contacts, &mut output).await;
    summary_output.push_str(&format!("| Helpful Contacts | {} | {} | {} |\n", summary_helpful.yearly_count, summary_helpful.monthly_count, summary_helpful.status_emoji()));

    let summary_home_health = check_links_and_add_to_output("Home health services", topics.home_health_services, &mut output).await;
    summary_output.push_str(&format!("| Home health services | {} | {} | {} |\n", summary_home_health.yearly_count, summary_home_health.monthly_count, summary_home_health.status_emoji()));

    let summary_hospice = check_links_and_add_to_output("Hospice care", topics.hospice_care, &mut output).await;
    summary_output.push_str(&format!("| Hospice care | {} | {} | {} |\n", summary_hospice.yearly_count, summary_hospice.monthly_count, summary_hospice.status_emoji()));

    let summary_hospitals = check_links_and_add_to_output("Hospitals", topics.hospitals, &mut output).await;
    summary_output.push_str(&format!("| Hospitals | {} | {} | {} |\n", summary_hospitals.yearly_count, summary_hospitals.monthly_count, summary_hospitals.status_emoji()));

    let summary_rehabilitation = check_links_and_add_to_output("Inpatient rehabilitation facilities", topics.inpatient_rehabilitation_facilities, &mut output).await;
    summary_output.push_str(&format!("| Inpatient rehabilitation facilities | {} | {} | {} |\n", summary_rehabilitation.yearly_count, summary_rehabilitation.monthly_count, summary_rehabilitation.status_emoji()));

    let summary_long_term = check_links_and_add_to_output("Long-term care hospitals", topics.long_term_care_hospitals, &mut output).await;
    summary_output.push_str(&format!("| Long-term care hospitals | {} | {} | {} |\n", summary_long_term.yearly_count, summary_long_term.monthly_count, summary_long_term.status_emoji()));

    let summary_nursing = check_links_and_add_to_output("Nursing homes including rehab services", topics.nursing_homes_including_rehab_services, &mut output).await;
    summary_output.push_str(&format!("| Nursing homes including rehab services | {} | {} | {} |\n", summary_nursing.yearly_count, summary_nursing.monthly_count, summary_nursing.status_emoji()));

    let summary_nh_backup = check_links_and_add_to_output("nh-backup", topics.nh_backup, &mut output).await;
    summary_output.push_str(&format!("| nh-backup | {} | {} | {} |\n", summary_nh_backup.yearly_count, summary_nh_backup.monthly_count, summary_nh_backup.status_emoji()));

    let summary_physician = check_links_and_add_to_output("Physician office visit costs", topics.physician_office_visit_costs, &mut output).await;
    summary_output.push_str(&format!("| Physician office visit costs | {} | {} | {} |\n", summary_physician.yearly_count, summary_physician.monthly_count, summary_physician.status_emoji()));

    let summary_supplier = check_links_and_add_to_output("Supplier directory", topics.supplier_directory, &mut output).await;
    summary_output.push_str(&format!("| Supplier directory | {} | {} | {} |\n", summary_supplier.yearly_count, summary_supplier.monthly_count, summary_supplier.status_emoji()));

    // Append the summary report below the `# Archive Report` heading.
    output = format!("# Archive Report\n\n{}\n\n{}", summary_output, output);

    // Write to Archives.md file
    let mut file = match File::create("Archives.md") {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to create Archives.md file: {:?}", e);
            return Err(Box::new(e));
        }
    };

    if let Err(e) = file.write_all(output.as_bytes()) {
        error!("Failed to write to Archives.md file: {:?}", e);
        return Err(Box::new(e));
    }

    info!("File Archives.md was successfully created and written to.");

    Ok(())
}

async fn check_links_and_add_to_output(
    category: &str,
    main_map: Option<YearMap>,
    output: &mut String,
) -> Summary {
    let mut summary = Summary::new();

    if let Some(main_map) = main_map {
        info!("Processing category: {}", category);
        output.push_str(&format!("## {}\n\n", category));

        let mut sorted_years: Vec<_> = main_map.years.iter().collect();
        sorted_years.sort_by(|a, b| b.0.cmp(a.0)); // Sort years descending

        for (year, entries) in sorted_years {
            info!("Processing year: {}", year);
            output.push_str(&format!("### {} archived data snapshots\n\n", year));

            let mut yearly_archive = None;
            let mut monthly_archives = Vec::new();

            for entry in entries {
                if entry.url.contains("_archive_") || entry.entry_type.as_deref() == Some("Annual") {
                    let status = is_url_reachable(&entry.url).await;
                    summary.add_yearly(status);
                    yearly_archive = Some(entry.formatted_yearly_entry().await);
                } else {
                    let status = is_url_reachable(&entry.url).await;
                    summary.add_monthly(status);
                    monthly_archives.push(entry);
                }
            }

            if let Some(yearly) = yearly_archive {
                output.push_str(&format!("{}\n\n", yearly));
            }

            if !monthly_archives.is_empty() {
                monthly_archives.sort_by(|a, b| {
                    a.month
                        .as_ref()
                        .unwrap_or(&"12".to_string())
                        .cmp(&b.month.as_ref().unwrap_or(&"12".to_string()))
                });

                let all_reachable = join_all(
                    monthly_archives.iter().map(|entry| is_url_reachable(&entry.url))
                ).await.into_iter().all(|reachable| reachable);

                let emoji = if all_reachable { "✅" } else { "❌" };

                output.push_str(&format!("{} **Monthly Archives**\n\n", emoji));
                output.push_str("| File | Release Date | Size | Status |\n");
                output.push_str("|-----------|-----------|-----------|-----------|\n");

                for entry in monthly_archives {
                    output.push_str(&format!("{}\n", entry.formatted_entry().await));
                }
            }
            output.push('\n');
        }
    } else {
        warn!("No entries found for category: {}", category);
    }

    summary
}
