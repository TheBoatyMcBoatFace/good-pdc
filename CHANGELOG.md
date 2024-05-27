# CHANGELOG

## [1.1.0] - Data Insight - 2024-05-27

### Added
- **New Files**:
  - `src/archive_report.rs`: Added to handle archive report generation.
  - `src/dataset_report.rs`: Added to handle dataset report generation.
  - `src/utils.rs`: Added to provide utility functions like URL reachability checks.
- **New Directory**:
  - `datasets/`: Added directory to store dataset report files.
- **GitHub Actions**:
  - Added workflow dispatch to `.github/workflows/link_checker.yml`.
  - Added `SENTRY_DSN` environment variable setup in `.github/workflows/link_checker.yml`.
  - Added `LOG_LEVEL` environment variable setup in `.github/workflows/link_checker.yml`.
  - Added `datasets/*.md` to verification and commit steps in `.github/workflows/link_checker.yml`.

### Changed
- **Code Refactors**:
  - Refactored link checking code to use async functions in `src/archive_report.rs`.
  - Implemented logging and error tracking using the tracing and sentry crates across the codebase, including `archive_report.rs`, `dataset_report.rs`, `utils.rs`, and `main.rs`.
  - Modularized the code by separating archive and dataset report generation into individual modules (`src/archive_report.rs`, `src/dataset_report.rs`).

### Updated
- **README.md**:
  - Updated to include links and details about dataset reports generated on `datasets/README.md`.
  - Added sections to explain the functionality and how it works.
  - Updated the project status buttons and badges.
- **README in `datasets/` Directory**:
  - Created `datasets/README.md` to detail the available datasets and link check results.
  - Included the dataset report structure and automated check information.
- **Archives.md**:
  - Updated with the most recent data and generated baseline reports for all dataset topics.

### Dependencies
- **Cargo.toml**:
  - Added new dependencies for handling dataset reports, logging, and error tracking:
    - `tokio`
    - `csv`
    - `uuid`
    - `chrono`
    - `futures`
    - `sentry`
    - `tracing-subscriber`
    - `tracing`
- **Cargo Package**:
  - Updated package name to `good-pdc`.

### Fixed
- **Bug Fixes**:
  - Corrected URL reachability checks to handle async requests.
  - Ensured proper error handling and reporting using Sentry.

### Removed
- **Redundant Code and Files**:
  - Removed the `workflow_run` triggering (commented `"Link Checker"` workflow) from `check_archive.yml`.

### Miscellaneous
- **Refactoring**:
  - Improved code structure with better modularization and error handling.
  - Enhanced logging for better traceability and debug support.

## [1.0.0] - Initial release - 2024-05-25
- Initial implementation of link checking workflow on the `main` branch.
- Basic Rust setup to check links and generate `Archives.md`.
- GitHub Actions setup for link verification and auto-commit.
