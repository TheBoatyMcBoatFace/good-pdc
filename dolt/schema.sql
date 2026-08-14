-- Schema for the Dolt mirror of good-pdc results (DoltHub: kingfish/CMS-PDC).
--
-- good-pdc rewrites these current-state tables on each run; Dolt's own commit
-- history is what captures how the data changes over time. Apply this once when
-- first setting up the Dolt repo (the app also creates the tables if missing):
--
--   dolt clone kingfish/CMS-PDC dolt-data
--   cd dolt-data && dolt sql < ../dolt/schema.sql

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
);
