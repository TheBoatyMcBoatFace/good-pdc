// src/dataset_report.rs

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, create_dir_all};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use sentry::add_breadcrumb;
use sentry::Breadcrumb;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::utils::{self, LinkStatus};

/// Where per-dataset download stats are cached between runs so we only pull the
/// full files about once a day instead of on every 3-hourly link check.
const STATS_CACHE_PATH: &str = "datasets/.stats-cache.json";
const DEFAULT_REFRESH_HOURS: i64 = 24;

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
    ("Supplier directory", "SUP"),
];

/// Real data-integrity results computed from the downloaded CSV.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Integrity {
    columns_consistent: bool,
    bad_row_count: usize,
    header_valid: bool,
    header_issue: Option<String>,
    is_utf8: bool,
}

impl Integrity {
    fn column_cell(&self) -> String {
        if self.columns_consistent {
            "✅".to_string()
        } else {
            format!("❌ ({} rows differ)", self.bad_row_count)
        }
    }

    fn header_cell(&self) -> String {
        if self.header_valid {
            "✅".to_string()
        } else {
            format!("❌ {}", self.header_issue.as_deref().unwrap_or("invalid"))
        }
    }

    fn encoding_cell(&self) -> &'static str {
        if self.is_utf8 {
            "✅ UTF-8"
        } else {
            "⚠️ Not UTF-8"
        }
    }

    fn all_passing(&self) -> bool {
        self.columns_consistent && self.header_valid && self.is_utf8
    }
}

/// Statistics + integrity results for a single downloaded dataset file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatasetStats {
    downloaded_at: DateTime<Utc>,
    filesize_mb: String,
    row_count: usize,
    column_count: usize,
    #[serde(flatten)]
    integrity: Integrity,
}

/// Cache of per-dataset stats keyed by dataset identifier.
type StatsCache = HashMap<String, DatasetStats>;

/// A fully processed dataset ready to be written to its topic file.
struct ProcessedDataset {
    topic: &'static str,
    id: String,
    block: String,
    stats: Option<DatasetStats>,
}

/// Generate the per-topic dataset reports.
pub async fn generate_dataset_report(client: &Client) -> Result<()> {
    let url = "https://data.cms.gov/provider-data/api/1/metastore/schemas/dataset/items?show-reference-ids=false";

    add_breadcrumb(Breadcrumb {
        message: Some("Fetching datasets".into()),
        ..Default::default()
    });

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        let msg = format!(
            "Failed to fetch datasets from {}: HTTP {}",
            url,
            response.status()
        );
        error!("{}", msg);
        sentry::capture_message(&msg, sentry::Level::Error);
        return Err(anyhow!(msg));
    }

    info!("Dataset response received!");
    let datasets: Vec<Dataset> = response.json().await.context("parsing dataset list")?;

    let concurrency = utils::max_concurrency();
    let refresh = refresh_hours();
    let cache = Arc::new(load_stats_cache());
    info!(
        "Processing {} datasets ({} at a time, re-downloading files older than {}h)...",
        datasets.len(),
        concurrency,
        refresh
    );

    // Process datasets concurrently (bounded), then regenerate topic files from
    // the collected results so each run reflects exactly the current API state.
    let processed: Vec<ProcessedDataset> = stream::iter(datasets)
        .map(|dataset| {
            let client = client.clone();
            let cache = Arc::clone(&cache);
            async move { process_dataset(&client, dataset, &cache, refresh).await }
        })
        .buffer_unordered(concurrency)
        .filter_map(|result| async move {
            match result {
                Ok(processed) => Some(processed),
                Err(e) => {
                    error!("Error processing dataset: {:?}", e);
                    sentry::capture_message(
                        &format!("Error processing dataset: {:?}", e),
                        sentry::Level::Error,
                    );
                    None
                }
            }
        })
        .collect()
        .await;

    write_topic_files(&processed)?;

    // Persist refreshed stats so the next run can skip downloads that are still
    // within the refresh window.
    let updated_cache: StatsCache = processed
        .iter()
        .filter_map(|p| p.stats.clone().map(|s| (p.id.clone(), s)))
        .collect();
    if let Err(e) = save_stats_cache(&updated_cache) {
        warn!("Failed to write stats cache: {:?}", e);
    }

    info!(
        "All datasets processed. Wrote reports for {} datasets.",
        processed.len()
    );
    Ok(())
}

/// Hours before a cached dataset download is considered stale. Override with
/// `DATASET_REFRESH_HOURS` (set to 0 to force a download every run).
fn refresh_hours() -> i64 {
    std::env::var("DATASET_REFRESH_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|h| *h >= 0)
        .unwrap_or(DEFAULT_REFRESH_HOURS)
}

/// Load the persisted stats cache; returns an empty cache if missing/invalid.
fn load_stats_cache() -> StatsCache {
    match fs::read_to_string(STATS_CACHE_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            warn!("Ignoring unreadable stats cache: {:?}", e);
            StatsCache::new()
        }),
        Err(_) => StatsCache::new(),
    }
}

/// Persist the stats cache (pretty-printed for reviewable diffs).
fn save_stats_cache(cache: &StatsCache) -> Result<()> {
    create_dir_all("datasets")?;
    let json = serde_json::to_string_pretty(cache)?;
    fs::write(STATS_CACHE_PATH, json)?;
    Ok(())
}

/// Resolve a dataset's topic code (e.g. "DAC") from its themes.
fn resolve_topic(themes: &[Theme]) -> &'static str {
    for theme in themes {
        if let Some((_, code)) = DATA_TOPICS.iter().find(|(name, _)| *name == theme.data) {
            return code;
        }
    }
    "undefined"
}

/// Decide whether a dataset file needs re-downloading: true when there's no
/// cached stats or the cached copy is older than the refresh window.
fn needs_download(cached: Option<&DatasetStats>, refresh_hours: i64, now: DateTime<Utc>) -> bool {
    match cached {
        Some(s) => now.signed_duration_since(s.downloaded_at) >= Duration::hours(refresh_hours),
        None => true,
    }
}

/// Check all links for a dataset, gather stats, and build its Markdown block.
async fn process_dataset(
    client: &Client,
    dataset: Dataset,
    cache: &StatsCache,
    refresh_hours: i64,
) -> Result<ProcessedDataset> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Processing dataset: {}", dataset.title)),
        ..Default::default()
    });

    let topic = resolve_topic(&dataset.theme);

    let download_url = dataset
        .distribution
        .first()
        .and_then(|dist| dist.distribution_data.download_url.as_deref());

    let download_status = match download_url {
        Some(url) => {
            debug!("Checking download URL: {}", url);
            utils::check_link(client, url).await
        }
        None => {
            warn!("No download URL found for dataset: {}", dataset.title);
            LinkStatus::Broken
        }
    };

    let landing_status = utils::check_link(client, &dataset.landing_page).await;
    let pdc_page = format!("https://data.cms.gov/provider-data/dataset/{}", dataset.id);
    let pdc_status = utils::check_link(client, &pdc_page).await;

    // Reuse cached stats when still fresh; only download the (possibly multi-GB)
    // file when the cache is missing or older than the refresh window.
    let cached = cache.get(&dataset.id);
    let should_download = needs_download(cached, refresh_hours, Utc::now());

    let stats = if let Some(url) = download_url {
        if download_status.is_ok() && should_download {
            match get_dataset_stats(client, url).await {
                Ok(stats) => Some(stats),
                Err(e) => {
                    warn!(
                        "Failed to compute statistics for {}: {:?}",
                        dataset.title, e
                    );
                    cached.cloned()
                }
            }
        } else {
            // Fresh enough, or the link is down: keep the last known stats.
            cached.cloned()
        }
    } else {
        cached.cloned()
    };

    let block = build_block(
        &dataset,
        download_url,
        download_status,
        landing_status,
        pdc_status,
        &pdc_page,
        stats.as_ref(),
    );

    info!("Report built for dataset: {}", dataset.title);
    Ok(ProcessedDataset {
        topic,
        id: dataset.id,
        block,
        stats,
    })
}

/// Download a dataset file and compute size, row/column counts, and real
/// data-integrity results. The file is analyzed in memory (no temp files).
async fn get_dataset_stats(client: &Client, url: &str) -> Result<DatasetStats> {
    add_breadcrumb(Breadcrumb {
        message: Some(format!("Getting dataset statistics for URL: {}", url)),
        ..Default::default()
    });

    let response = client.get(url).send().await?;
    let content = response.bytes().await?;

    let filesize_mb = format!("{:.1}", content.len() as f64 / 1_000_000.0);
    let is_utf8 = std::str::from_utf8(&content).is_ok();
    let (row_count, column_count, integrity) = analyze_csv(&content, is_utf8);

    debug!(
        "Stats for {}: {} MB, {} rows, {} cols, utf8={}",
        url, filesize_mb, row_count, column_count, is_utf8
    );

    Ok(DatasetStats {
        downloaded_at: Utc::now(),
        filesize_mb,
        row_count,
        column_count,
        integrity,
    })
}

/// Inspect the CSV bytes: header validity, column-count consistency, row count.
fn analyze_csv(content: &[u8], is_utf8: bool) -> (usize, usize, Integrity) {
    // `flexible(true)` lets us count rows with mismatched column counts instead
    // of aborting the read, so we can report the inconsistency truthfully.
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content);

    let (column_count, header_valid, header_issue) = match reader.byte_headers() {
        Ok(headers) => {
            let cols = headers.len();
            let values: Vec<String> = headers
                .iter()
                .map(|h| String::from_utf8_lossy(h).trim().to_string())
                .collect();

            let has_empty = values.iter().any(|h| h.is_empty());
            let mut seen = HashSet::new();
            let has_duplicate = values.iter().any(|h| !seen.insert(h.to_lowercase()));

            let issue = if cols == 0 {
                Some("no header row".to_string())
            } else if has_empty {
                Some("empty header cell".to_string())
            } else if has_duplicate {
                Some("duplicate header".to_string())
            } else {
                None
            };
            (cols, issue.is_none(), issue)
        }
        Err(_) => (0, false, Some("unreadable header".to_string())),
    };

    let mut row_count = 0usize;
    let mut bad_row_count = 0usize;
    for record in reader.byte_records() {
        match record {
            Ok(record) => {
                row_count += 1;
                if column_count > 0 && record.len() != column_count {
                    bad_row_count += 1;
                }
            }
            Err(e) => {
                bad_row_count += 1;
                debug!("CSV record failed to read: {:?}", e);
            }
        }
    }

    let integrity = Integrity {
        columns_consistent: bad_row_count == 0,
        bad_row_count,
        header_valid,
        header_issue,
        is_utf8,
    };

    (row_count, column_count, integrity)
}

/// Build the Markdown block for one dataset, wrapped in stable markers so the
/// report is regenerated cleanly on every run.
fn build_block(
    dataset: &Dataset,
    download_url: Option<&str>,
    download_status: LinkStatus,
    landing_status: LinkStatus,
    pdc_status: LinkStatus,
    pdc_page: &str,
    stats: Option<&DatasetStats>,
) -> String {
    let (filesize, row_count, column_count) = match stats {
        Some(s) => (
            s.filesize_mb.clone(),
            s.row_count.to_string(),
            s.column_count.to_string(),
        ),
        None => ("N/A".to_string(), "N/A".to_string(), "N/A".to_string()),
    };

    let file_downloaded = match stats {
        Some(s) => s.downloaded_at.format("%Y-%m-%d").to_string(),
        None => "N/A".to_string(),
    };

    let (integrity_summary, column_cell, header_cell, encoding_cell) = match stats {
        Some(s) => (
            if s.integrity.all_passing() {
                "✅"
            } else {
                "⚠️"
            },
            s.integrity.column_cell(),
            s.integrity.header_cell(),
            s.integrity.encoding_cell().to_string(),
        ),
        None => (
            "N/A",
            "N/A".to_string(),
            "N/A".to_string(),
            "N/A".to_string(),
        ),
    };

    let mut block = String::new();

    block.push_str(&format!("<!-- dataset:{}:start -->\n", dataset.id));
    block.push_str(&format!(
        "## {}\n{}\n\n**Dataset ID:** {}\n\n**Status:** {}\n\n### Dataset Details\n\n",
        dataset.title,
        dataset.description,
        dataset.id,
        download_status.emoji()
    ));

    block.push_str(&format!(
        "<details>\n<summary>File History</summary>\n\n|  Activity   |  Description |  Date  |\n| --- | --- | --- |\n| Issued Date   | When the dataset was created | {} |\n| Modified Date | when it was last modified | {} |\n| Release Date | when the dataset was made public | {} |\n| Last Checked | when this dataset was last tested | {} |\n\n</details>\n\n",
        dataset.issued,
        dataset.modified,
        dataset.released,
        chrono::Utc::now().format("%Y-%m-%d")
    ));

    block.push_str(&format!(
        "<details>\n<summary>File Overview</summary>\n\n| Metric | Result |\n| --- | --- |\n| Filesize | {} MB |\n| Row Count | {} |\n| Column Count | {} |\n| File Downloaded | {} |\n\n</details>\n\n",
        filesize, row_count, column_count, file_downloaded
    ));

    block.push_str(&format!(
        "### Data Integrity Tests\nDoes this dataset abide by basic data formatting standards?\n<details>\n\n<summary>{} </summary>\n\n| Test | Description | Result |\n| --- | --- | --- |\n| Column Count Consistency | Verify that all rows have the same number of columns. | {} |\n| Header Validation | Ensure the CSV has a header row and all headers are unique and meaningful. | {} |\n| Encoding Validation | Verify that the CSV file uses UTF-8 encoding. | {} |\n\n</details>\n\n",
        integrity_summary, column_cell, header_cell, encoding_cell
    ));

    block.push_str(
        "### Public Access Tests\nTesting the additional resources listed in the api.\n\n",
    );
    block.push_str(
        "| Page | Status | A11y Test |\n| :-----------: | :-----------: | :-----------: |\n",
    );

    block.push_str(&format!(
        "| [PDC Page]({}) | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
        pdc_page, pdc_status.emoji(), pdc_page, pdc_page
    ));
    block.push_str(&format!(
        "| [Landing Page]({}) | {} | [![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl={})](https://validator.nu/?doc={}) |\n",
        dataset.landing_page, landing_status.emoji(), dataset.landing_page, dataset.landing_page
    ));
    block.push_str(&format!(
        "| [Direct Download]({}) | {} |  |\n",
        download_url.unwrap_or("#"),
        download_status.emoji()
    ));

    block.push_str(&format!("\n<!-- dataset:{}:end -->\n", dataset.id));

    block
}

/// Regenerate every topic file from the processed datasets. Rewriting the whole
/// file each run avoids stale entries and the write races of the old approach.
fn write_topic_files(datasets: &[ProcessedDataset]) -> Result<()> {
    create_dir_all("datasets")?;

    let mut by_topic: BTreeMap<&str, Vec<&ProcessedDataset>> = BTreeMap::new();
    for dataset in datasets {
        by_topic.entry(dataset.topic).or_default().push(dataset);
    }

    for (topic, mut items) in by_topic {
        // Stable ordering keeps git diffs meaningful between runs.
        items.sort_by(|a, b| a.id.cmp(&b.id));

        let file_path = format!("datasets/{}.md", topic);
        let mut output = String::new();
        output.push_str(&format!("# {} Datasets\n", topic));
        output.push_str(&format!(
            "Testing all the {} datasets listed on the Provider Data Catalog (PDC) API.\n\n",
            topic
        ));

        for item in items {
            output.push_str(&item.block);
            output.push('\n');
        }

        fs::write(&file_path, output).with_context(|| format!("writing report {}", file_path))?;
        info!("Wrote report at {}", file_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_csv_passes_all_checks() {
        let csv = b"name,age,city\nAlice,30,NYC\nBob,25,LA\n";
        let (rows, cols, integrity) = analyze_csv(csv, true);
        assert_eq!(rows, 2);
        assert_eq!(cols, 3);
        assert!(integrity.columns_consistent);
        assert_eq!(integrity.bad_row_count, 0);
        assert!(integrity.header_valid);
        assert!(integrity.all_passing());
    }

    #[test]
    fn inconsistent_column_counts_are_flagged() {
        // Second data row is missing a column.
        let csv = b"a,b,c\n1,2,3\n4,5\n6,7,8\n";
        let (rows, cols, integrity) = analyze_csv(csv, true);
        assert_eq!(rows, 3);
        assert_eq!(cols, 3);
        assert!(!integrity.columns_consistent);
        assert_eq!(integrity.bad_row_count, 1);
    }

    #[test]
    fn duplicate_headers_are_flagged() {
        let csv = b"id,name,id\n1,x,2\n";
        let (_, _, integrity) = analyze_csv(csv, true);
        assert!(!integrity.header_valid);
        assert_eq!(integrity.header_issue.as_deref(), Some("duplicate header"));
    }

    #[test]
    fn empty_header_cell_is_flagged() {
        let csv = b"id,,name\n1,2,3\n";
        let (_, _, integrity) = analyze_csv(csv, true);
        assert!(!integrity.header_valid);
        assert_eq!(integrity.header_issue.as_deref(), Some("empty header cell"));
    }

    #[test]
    fn encoding_flag_is_reported_truthfully() {
        let csv = b"a,b\n1,2\n";
        let (_, _, integrity) = analyze_csv(csv, false);
        assert!(!integrity.is_utf8);
        assert_eq!(integrity.encoding_cell(), "⚠️ Not UTF-8");
        assert!(!integrity.all_passing());
    }

    #[test]
    fn resolve_topic_maps_known_and_unknown_themes() {
        let known = vec![Theme {
            data: "Hospitals".to_string(),
        }];
        assert_eq!(resolve_topic(&known), "HOS");

        let unknown = vec![Theme {
            data: "Something else".to_string(),
        }];
        assert_eq!(resolve_topic(&unknown), "undefined");
    }

    fn stats_downloaded_at(when: DateTime<Utc>) -> DatasetStats {
        DatasetStats {
            downloaded_at: when,
            filesize_mb: "1.0".to_string(),
            row_count: 1,
            column_count: 1,
            integrity: Integrity {
                columns_consistent: true,
                bad_row_count: 0,
                header_valid: true,
                header_issue: None,
                is_utf8: true,
            },
        }
    }

    #[test]
    fn missing_cache_always_downloads() {
        assert!(needs_download(None, 24, Utc::now()));
    }

    #[test]
    fn fresh_cache_skips_download() {
        let now = Utc::now();
        let recent = stats_downloaded_at(now - Duration::hours(1));
        assert!(!needs_download(Some(&recent), 24, now));
    }

    #[test]
    fn stale_cache_triggers_download() {
        let now = Utc::now();
        let old = stats_downloaded_at(now - Duration::hours(25));
        assert!(needs_download(Some(&old), 24, now));
    }

    #[test]
    fn zero_refresh_window_always_downloads() {
        let now = Utc::now();
        let recent = stats_downloaded_at(now - Duration::minutes(1));
        assert!(needs_download(Some(&recent), 0, now));
    }
}
