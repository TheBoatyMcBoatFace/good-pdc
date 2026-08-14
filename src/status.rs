// src/status.rs
//
// Aggregates the results of the dataset and archive checks into a single
// machine-readable `status.json` plus a human-friendly `STATUS.md` dashboard,
// and injects a rollup table into `datasets/README.md`.

use std::fs;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::{info, warn};

const STATUS_JSON_PATH: &str = "status.json";
const STATUS_MD_PATH: &str = "STATUS.md";
const DATASETS_README_PATH: &str = "datasets/README.md";
const MARKER_START: &str = "<!-- STATUS:START -->";
const MARKER_END: &str = "<!-- STATUS:END -->";

/// One dataset's health at a glance.
#[derive(Debug, Clone, Serialize)]
pub struct DatasetStatusEntry {
    pub id: String,
    pub title: String,
    pub topic: String,
    pub download_ok: bool,
    pub landing_ok: bool,
    pub pdc_ok: bool,
    /// `None` when the file wasn't downloaded (no stats to judge integrity).
    pub integrity_ok: Option<bool>,
    pub healthy: bool,
}

impl DatasetStatusEntry {
    pub fn new(
        id: String,
        title: String,
        topic: String,
        download_ok: bool,
        landing_ok: bool,
        pdc_ok: bool,
        integrity_ok: Option<bool>,
    ) -> Self {
        // A dataset is healthy when every public link resolves and integrity
        // (when we have it) passes. Missing integrity doesn't fail the dataset.
        let healthy = download_ok && landing_ok && pdc_ok && integrity_ok != Some(false);
        Self {
            id,
            title,
            topic,
            download_ok,
            landing_ok,
            pdc_ok,
            integrity_ok,
            healthy,
        }
    }
}

/// Per-topic dataset counts.
#[derive(Debug, Clone, Serialize)]
pub struct TopicRollup {
    pub topic: String,
    pub total: usize,
    pub healthy: usize,
    pub broken: usize,
}

#[derive(Debug, Serialize)]
pub struct DatasetsStatus {
    pub total: usize,
    pub healthy: usize,
    pub broken: usize,
    pub by_topic: Vec<TopicRollup>,
    pub entries: Vec<DatasetStatusEntry>,
}

impl DatasetsStatus {
    pub fn from_entries(mut entries: Vec<DatasetStatusEntry>) -> Self {
        entries.sort_by(|a, b| a.id.cmp(&b.id));

        let total = entries.len();
        let healthy = entries.iter().filter(|e| e.healthy).count();
        let broken = total - healthy;

        // Build per-topic rollups in stable (alphabetical) topic order.
        let mut topics: Vec<String> = entries.iter().map(|e| e.topic.clone()).collect();
        topics.sort();
        topics.dedup();

        let by_topic = topics
            .into_iter()
            .map(|topic| {
                let items: Vec<&DatasetStatusEntry> =
                    entries.iter().filter(|e| e.topic == topic).collect();
                let t_total = items.len();
                let t_healthy = items.iter().filter(|e| e.healthy).count();
                TopicRollup {
                    topic,
                    total: t_total,
                    healthy: t_healthy,
                    broken: t_total - t_healthy,
                }
            })
            .collect();

        Self {
            total,
            healthy,
            broken,
            by_topic,
            entries,
        }
    }
}

/// One archive topic's health.
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveTopicStatus {
    pub topic: String,
    pub yearly_count: usize,
    pub monthly_count: usize,
    pub healthy: bool,
}

#[derive(Debug, Serialize)]
pub struct ArchivesStatus {
    pub healthy: bool,
    pub by_topic: Vec<ArchiveTopicStatus>,
}

impl ArchivesStatus {
    pub fn from_topics(by_topic: Vec<ArchiveTopicStatus>) -> Self {
        let healthy = by_topic.iter().all(|t| t.healthy);
        Self { healthy, by_topic }
    }

    fn healthy_count(&self) -> usize {
        self.by_topic.iter().filter(|t| t.healthy).count()
    }
}

/// The full result of one run.
#[derive(Debug, Serialize)]
pub struct RunStatus {
    pub generated_at: DateTime<Utc>,
    pub healthy: bool,
    pub datasets: DatasetsStatus,
    pub archives: ArchivesStatus,
}

impl RunStatus {
    pub fn new(datasets: DatasetsStatus, archives: ArchivesStatus) -> Self {
        let healthy = datasets.broken == 0 && archives.healthy;
        // Truncate to the day so the committed status files only change on real
        // data changes or once per day — not on every run's timestamp.
        let generated_at = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("valid midnight")
            .and_utc();
        Self {
            generated_at,
            healthy,
            datasets,
            archives,
        }
    }
}

fn yn(ok: bool) -> &'static str {
    if ok {
        "✅"
    } else {
        "❌"
    }
}

fn tri(ok: Option<bool>) -> &'static str {
    match ok {
        Some(true) => "✅",
        Some(false) => "❌",
        None => "—",
    }
}

/// Write `status.json`, render `STATUS.md`, and inject the datasets rollup.
pub fn write_all(run: &RunStatus) -> Result<()> {
    write_status_json(run)?;
    write_status_md(run)?;
    inject_datasets_rollup(run)?;
    Ok(())
}

fn write_status_json(run: &RunStatus) -> Result<()> {
    let json = serde_json::to_string_pretty(run)?;
    fs::write(STATUS_JSON_PATH, json).context("writing status.json")?;
    info!("Wrote {}", STATUS_JSON_PATH);
    Ok(())
}

fn write_status_md(run: &RunStatus) -> Result<()> {
    let d = &run.datasets;
    let a = &run.archives;
    let stamp = run.generated_at.format("%Y-%m-%d");

    let mut md = String::new();
    md.push_str("# good-pdc Status 🩺\n\n");
    md.push_str(&format!("_Last checked: {}_\n\n", stamp));

    let headline = if run.healthy {
        "✅ **All systems nominal.** Every tracked link resolves and the data passes its integrity checks."
    } else {
        "❌ **Heads up — some things need attention.** See the tables below for the culprits."
    };
    md.push_str(&format!("{}\n\n", headline));

    md.push_str("## TL;DR\n\n");
    md.push_str("| Check | ✅ Healthy | ❌ Broken | Total |\n");
    md.push_str("| --- | :---: | :---: | :---: |\n");
    md.push_str(&format!(
        "| Datasets | {} | {} | {} |\n",
        d.healthy, d.broken, d.total
    ));
    md.push_str(&format!(
        "| Archive topics | {} | {} | {} |\n\n",
        a.healthy_count(),
        a.by_topic.len() - a.healthy_count(),
        a.by_topic.len()
    ));

    md.push_str("## Datasets by topic\n\n");
    md.push_str("| Topic | ✅ Healthy | ❌ Broken | Total |\n");
    md.push_str("| --- | :---: | :---: | :---: |\n");
    for t in &d.by_topic {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            t.topic, t.healthy, t.broken, t.total
        ));
    }
    md.push('\n');

    md.push_str("## Archive topics\n\n");
    md.push_str("| Topic | Yearly | Monthly | Status |\n");
    md.push_str("| --- | :---: | :---: | :---: |\n");
    for t in &a.by_topic {
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            t.topic,
            t.yearly_count,
            t.monthly_count,
            yn(t.healthy)
        ));
    }
    md.push('\n');

    let broken: Vec<&DatasetStatusEntry> = d.entries.iter().filter(|e| !e.healthy).collect();
    if broken.is_empty() {
        md.push_str("## Needs attention\n\nNothing! 🎉 Everything checks out.\n");
    } else {
        md.push_str("## Needs attention\n\n");
        md.push_str("| Dataset | Topic | Download | Landing | PDC | Integrity |\n");
        md.push_str("| --- | --- | :---: | :---: | :---: | :---: |\n");
        for e in broken {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                e.title,
                e.topic,
                yn(e.download_ok),
                yn(e.landing_ok),
                yn(e.pdc_ok),
                tri(e.integrity_ok)
            ));
        }
        md.push('\n');
    }

    md.push_str("\n_Generated automatically by [good-pdc](https://github.com/TheBoatyMcBoatFace/good-pdc). Machine-readable version: [`status.json`](status.json)._\n");

    fs::write(STATUS_MD_PATH, md).context("writing STATUS.md")?;
    info!("Wrote {}", STATUS_MD_PATH);
    Ok(())
}

/// Build the compact rollup table injected into `datasets/README.md`.
fn datasets_rollup_table(run: &RunStatus) -> String {
    let d = &run.datasets;
    let stamp = run.generated_at.format("%Y-%m-%d");

    let mut table = String::new();
    table.push_str(&format!(
        "**{} of {} datasets healthy** as of {} — [full dashboard](../STATUS.md)\n\n",
        d.healthy, d.total, stamp
    ));
    table.push_str("| Topic | ✅ Healthy | ❌ Broken | Total |\n");
    table.push_str("| --- | :---: | :---: | :---: |\n");
    for t in &d.by_topic {
        table.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            t.topic, t.healthy, t.broken, t.total
        ));
    }
    table
}

/// Replace the content between the STATUS markers in `datasets/README.md`.
fn inject_datasets_rollup(run: &RunStatus) -> Result<()> {
    let content = match fs::read_to_string(DATASETS_README_PATH) {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "Skipping datasets README rollup ({}): {:?}",
                DATASETS_README_PATH, e
            );
            return Ok(());
        }
    };

    let (Some(start), Some(end)) = (content.find(MARKER_START), content.find(MARKER_END)) else {
        warn!(
            "STATUS markers not found in {}; skipping rollup injection",
            DATASETS_README_PATH
        );
        return Ok(());
    };

    let table = datasets_rollup_table(run);
    let mut updated = String::new();
    updated.push_str(&content[..start + MARKER_START.len()]);
    updated.push_str(&format!("\n{}\n", table));
    updated.push_str(&content[end..]);

    fs::write(DATASETS_README_PATH, updated).context("updating datasets/README.md")?;
    info!("Updated status rollup in {}", DATASETS_README_PATH);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_at_is_truncated_to_date() {
        // Committed artifacts must not churn on sub-day timestamps.
        let run = RunStatus::new(
            DatasetsStatus::from_entries(vec![]),
            ArchivesStatus::from_topics(vec![]),
        );
        assert_eq!(run.generated_at.format("%H:%M:%S").to_string(), "00:00:00");
    }

    #[test]
    fn healthy_when_all_links_and_integrity_pass() {
        let e = DatasetStatusEntry::new(
            "id".into(),
            "t".into(),
            "DAC".into(),
            true,
            true,
            true,
            Some(true),
        );
        assert!(e.healthy);
    }

    #[test]
    fn missing_integrity_does_not_fail_dataset() {
        let e = DatasetStatusEntry::new(
            "id".into(),
            "t".into(),
            "DAC".into(),
            true,
            true,
            true,
            None,
        );
        assert!(e.healthy);
    }

    #[test]
    fn failed_integrity_fails_dataset() {
        let e = DatasetStatusEntry::new(
            "id".into(),
            "t".into(),
            "DAC".into(),
            true,
            true,
            true,
            Some(false),
        );
        assert!(!e.healthy);
    }

    #[test]
    fn broken_link_fails_dataset() {
        let e = DatasetStatusEntry::new(
            "id".into(),
            "t".into(),
            "DAC".into(),
            false,
            true,
            true,
            Some(true),
        );
        assert!(!e.healthy);
    }

    #[test]
    fn rollups_count_per_topic() {
        let entries = vec![
            DatasetStatusEntry::new("b".into(), "B".into(), "DF".into(), true, true, true, None),
            DatasetStatusEntry::new(
                "a".into(),
                "A".into(),
                "DAC".into(),
                true,
                true,
                true,
                Some(true),
            ),
            DatasetStatusEntry::new(
                "c".into(),
                "C".into(),
                "DAC".into(),
                false,
                true,
                true,
                Some(true),
            ),
        ];
        let status = DatasetsStatus::from_entries(entries);

        assert_eq!(status.total, 3);
        assert_eq!(status.healthy, 2);
        assert_eq!(status.broken, 1);
        // Sorted by id, so first entry is "a".
        assert_eq!(status.entries[0].id, "a");

        let dac = status.by_topic.iter().find(|t| t.topic == "DAC").unwrap();
        assert_eq!(dac.total, 2);
        assert_eq!(dac.healthy, 1);
        assert_eq!(dac.broken, 1);
    }

    #[test]
    fn archives_status_unhealthy_if_any_topic_broken() {
        let topics = vec![
            ArchiveTopicStatus {
                topic: "A".into(),
                yearly_count: 1,
                monthly_count: 2,
                healthy: true,
            },
            ArchiveTopicStatus {
                topic: "B".into(),
                yearly_count: 0,
                monthly_count: 1,
                healthy: false,
            },
        ];
        let status = ArchivesStatus::from_topics(topics);
        assert!(!status.healthy);
        assert_eq!(status.healthy_count(), 1);
    }
}
