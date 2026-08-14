// src/dolt.rs
//
// Optional export of the current run into a Dolt database (DoltHub:
// kingfish/CMS-PDC). Entirely opt-in and non-fatal: when the `DOLT_*` env is
// not configured, nothing happens and the rest of the run is unaffected.
//
// The Dolt repo is treated as a working directory (from `dolt clone
// kingfish/CMS-PDC`). We rewrite the current-state tables each run and let
// Dolt's own commit history capture what changed over time.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use tracing::{info, warn};

use crate::status::{ArchiveTopicStatus, DatasetStatusEntry, RunStatus};

const SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS datasets (
    id VARCHAR(64) PRIMARY KEY,
    title TEXT,
    topic VARCHAR(16),
    download_ok BOOLEAN,
    landing_ok BOOLEAN,
    pdc_ok BOOLEAN,
    integrity_ok BOOLEAN,
    healthy BOOLEAN,
    last_checked DATETIME
);
CREATE TABLE IF NOT EXISTS archive_topics (
    topic VARCHAR(128) PRIMARY KEY,
    yearly_count INT,
    monthly_count INT,
    healthy BOOLEAN,
    last_checked DATETIME
);";

/// Whether the Dolt export is turned on.
fn enabled() -> bool {
    std::env::var("DOLT_ENABLED")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Whether to `dolt push` after committing (default off — commit locally only).
fn push_enabled() -> bool {
    std::env::var("DOLT_PUSH")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Working directory of the cloned Dolt repo (default `dolt-data`).
fn repo_dir() -> PathBuf {
    std::env::var("DOLT_REPO_DIR")
        .unwrap_or_else(|_| "dolt-data".to_string())
        .into()
}

fn remote_and_branch() -> (String, String) {
    let remote = std::env::var("DOLT_REMOTE").unwrap_or_else(|_| "origin".to_string());
    let branch = std::env::var("DOLT_BRANCH").unwrap_or_else(|_| "main".to_string());
    (remote, branch)
}

/// Export the run to Dolt. A no-op when disabled; never fails the overall run.
pub fn export(run: &RunStatus) -> Result<()> {
    if !enabled() {
        info!("Dolt export disabled (set DOLT_ENABLED=1 to turn it on).");
        return Ok(());
    }

    let dir = repo_dir();
    if !dir.join(".dolt").is_dir() {
        warn!(
            "DOLT_ENABLED is set but {} is not a Dolt repo. Run `dolt clone kingfish/CMS-PDC {}` first. Skipping.",
            dir.display(),
            dir.display()
        );
        return Ok(());
    }

    let script = build_script(run);
    run_dolt_sql(&dir, &script).context("running Dolt SQL batch")?;

    let message = format!(
        "good-pdc update {} ({} datasets, {} healthy)",
        run.generated_at.format("%Y-%m-%d %H:%M UTC"),
        run.datasets.total,
        run.datasets.healthy
    );
    commit(&dir, &message)?;

    if push_enabled() {
        let (remote, branch) = remote_and_branch();
        push(&dir, &remote, &branch)?;
    }

    info!("Dolt export complete.");
    Ok(())
}

/// Build the full SQL batch: schema, then a clean rewrite of both tables.
fn build_script(run: &RunStatus) -> String {
    let mut sql = String::new();
    sql.push_str(SCHEMA_SQL);
    sql.push('\n');

    let stamp = run.generated_at.format("%Y-%m-%d %H:%M:%S").to_string();

    sql.push_str("DELETE FROM datasets;\n");
    for entry in &run.datasets.entries {
        sql.push_str(&dataset_insert(entry, &stamp));
        sql.push('\n');
    }

    sql.push_str("DELETE FROM archive_topics;\n");
    for topic in &run.archives.by_topic {
        sql.push_str(&archive_insert(topic, &stamp));
        sql.push('\n');
    }

    sql
}

fn dataset_insert(entry: &DatasetStatusEntry, stamp: &str) -> String {
    format!(
        "INSERT INTO datasets (id, title, topic, download_ok, landing_ok, pdc_ok, integrity_ok, healthy, last_checked) VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {});",
        sql_str(&entry.id),
        sql_str(&entry.title),
        sql_str(&entry.topic),
        sql_bool(entry.download_ok),
        sql_bool(entry.landing_ok),
        sql_bool(entry.pdc_ok),
        sql_opt_bool(entry.integrity_ok),
        sql_bool(entry.healthy),
        sql_str(stamp),
    )
}

fn archive_insert(topic: &ArchiveTopicStatus, stamp: &str) -> String {
    format!(
        "INSERT INTO archive_topics (topic, yearly_count, monthly_count, healthy, last_checked) VALUES ({}, {}, {}, {}, {});",
        sql_str(&topic.topic),
        topic.yearly_count,
        topic.monthly_count,
        sql_bool(topic.healthy),
        sql_str(stamp),
    )
}

/// Quote and escape a string for a MySQL/Dolt SQL literal.
fn sql_str(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{}'", escaped)
}

fn sql_bool(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn sql_opt_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "1",
        Some(false) => "0",
        None => "NULL",
    }
}

fn run_dolt_sql(dir: &Path, script: &str) -> Result<()> {
    let mut child = Command::new("dolt")
        .arg("sql")
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning `dolt sql` (is the dolt CLI installed?)")?;

    child
        .stdin
        .take()
        .context("opening dolt stdin")?
        .write_all(script.as_bytes())
        .context("writing SQL to dolt")?;

    let output = child.wait_with_output().context("waiting for dolt")?;
    if !output.status.success() {
        bail!(
            "dolt sql failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn commit(dir: &Path, message: &str) -> Result<()> {
    run_dolt(dir, &["add", "datasets", "archive_topics"])?;

    // If nothing changed, `dolt commit` errors; treat that as a no-op.
    let output = Command::new("dolt")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .context("running `dolt commit`")?;

    if output.status.success() {
        info!("Committed Dolt changes: {}", message);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("nothing to commit") || stderr.contains("no changes") {
            info!("No Dolt changes to commit.");
        } else {
            bail!("dolt commit failed: {}", stderr.trim());
        }
    }
    Ok(())
}

fn push(dir: &Path, remote: &str, branch: &str) -> Result<()> {
    run_dolt(dir, &["push", remote, branch])?;
    info!("Pushed Dolt changes to {}/{}.", remote, branch);
    Ok(())
}

fn run_dolt(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("dolt")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("running `dolt {}`", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "dolt {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_sql_strings() {
        assert_eq!(sql_str("plain"), "'plain'");
        assert_eq!(sql_str("O'Brien"), "'O\\'Brien'");
        assert_eq!(sql_str("back\\slash"), "'back\\\\slash'");
    }

    #[test]
    fn optional_bool_maps_to_null() {
        assert_eq!(sql_opt_bool(Some(true)), "1");
        assert_eq!(sql_opt_bool(Some(false)), "0");
        assert_eq!(sql_opt_bool(None), "NULL");
    }

    #[test]
    fn dataset_insert_has_expected_shape() {
        let entry = DatasetStatusEntry::new(
            "abcd-1234".into(),
            "Nice, \"Quoted\" Title".into(),
            "DAC".into(),
            true,
            true,
            false,
            None,
        );
        let sql = dataset_insert(&entry, "2026-08-14 12:00:00");
        assert!(sql.starts_with("INSERT INTO datasets"));
        assert!(sql.contains("'abcd-1234'"));
        assert!(sql.contains("NULL")); // integrity_ok
        assert!(sql.ends_with(");"));
    }

    #[test]
    fn archive_insert_has_expected_shape() {
        let topic = ArchiveTopicStatus {
            topic: "Hospitals".into(),
            yearly_count: 3,
            monthly_count: 12,
            healthy: true,
        };
        let sql = archive_insert(&topic, "2026-08-14 12:00:00");
        assert!(sql.contains("'Hospitals'"));
        assert!(sql.contains(" 3, 12, 1, "));
    }
}
