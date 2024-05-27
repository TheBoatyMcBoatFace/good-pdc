# You good, PDC? 😎
![GitHub Repo stars](https://img.shields.io/github/stars/TheBoatyMcBoatFace/good-pdc)
![GitHub forks](https://img.shields.io/github/forks/TheBoatyMcBoatFace/good-pdc)

![GitHub License](https://img.shields.io/github/license/TheBoatyMcBoatFace/good-pdc)
![Last Commit](https://img.shields.io/github/last-commit/TheBoatyMcBoatFace/good-pdc)
[![Link Checker](https://github.com/TheBoatyMcBoatFace/good-pdc/actions/workflows/link_checker.yml/badge.svg)](https://github.com/TheBoatyMcBoatFace/good-pdc/actions/workflows/link_checker.yml)

This project periodically checks various links on CMS's Provider Data Catalog (PDC) to ensure that all links are operational and all data is accessible.

**THIS IS NOT AN OFFICIAL GOVERNMENT CODEBASE**.

Check the [Archives.md](Archives.md) file to see the status summary and detailed reports of each data topic archive.

To see the datasets, go to [Datasets README](datasets/README.md).

<div align="center">
  <a href="Archives.md">
    <img src="https://img.shields.io/badge/View-Archive_Link_Check_Results-purple?style=for-the-badge" alt="View Link Check Status">
  </a>
  <a href="datasets/README.md">
    <img src="https://img.shields.io/badge/View-Dataset_Link_Check_Results-blue?style=for-the-badge" alt="View Dataset Link Status">
  </a><br><br>
</div>

## What it do?

- **Link Validation**: We check CMS PDC links to make sure they're not ghosting 👻 us.

- **Categorized Reports**: We neatly categorize this info in `Archives.md` and associated dataset markdown files. _I'm really proud of how this turned out._
- **Dataset Analysis**: We test archive files and all datasets on the PDC. The datasets are analyzed, and basic checks are performed on them.
- **Public Data Check**: Uses the PDC API to find the archive and dataset files and then checks to make sure they exist.
- **Summarized Status**: A quick glance at the top tells you if things are going smoothly or if there's trouble.
- **Detailed Dataset Reports**: To view the dataset results, go to `datasets/README.md` where you can find links to various data topics.
- **GitHub Actions**: Automatically keeps things in check every time you push to the `main` branch. It also runs every three hours and sends notifications if there is a ❌ in the results file.
- **Error and Performance Monitoring**: Uses Sentry for error and performance monitoring. If you don't want to use it, just don't set `SENTRY_DSN` to any variable.
- **Logging Levels**: Select your log level by setting `LOG_LEVEL`. Options are `error`, `warn`, `info`, `debug`, `trace`.

## How It Works

### Dataset Reports

The dataset reports are generated using a Rust module that performs the following tasks:

1. **Fetching Datasets**
   - The module fetches a list of datasets from the PDC API.
   - Datasets are deserialized into a `Dataset` struct.

2. **Processing Datasets**
   - Each dataset is processed in parallel to improve efficiency.
   - The module checks the status of the dataset's download URL and landing page.

3. **Generating Reports**
   - The module constructs a markdown report for each dataset, including:
     - Dataset details (e.g., ID, title, description, issued date, modified date, release date).
     - File history and overview (e.g., filesize, row count, column count).
     - Data integrity tests (e.g., column count consistency, header validation, encoding validation).
     - Public access tests (e.g., status of PDC page, landing page, and direct download link).
   - Reports are saved to the `datasets` directory.

4. **Error Handling and Logging**
   - The module uses Sentry for error tracking and performance monitoring.
   - Detailed logging is performed using the `tracing` crate.

### Archive Reports

The archive reports are generated using another Rust module that performs the following tasks:

1. **Fetching Archives**
   - The module fetches a list of archive topics from the PDC API.
   - Archive topics are deserialized into a `Topics` struct.

2. **Processing Archives**
   - Each archive is processed to check the status of yearly and monthly archive URLs.
   - The module generates a summary of the yearly and monthly archive checks.

3. **Generating Reports**
   - The module constructs a markdown report for each archive topic, including:
     - Archive details and statuses.
     - Summary of yearly and monthly archive checks.
   - Reports are saved to the `Archives.md` file.

4. **Error Handling and Logging**
   - The module uses Sentry for error tracking and performance monitoring.
   - Detailed logging is performed using the `tracing` crate.


## Getting Started

1. **Clone the repo**:

    ```sh
    git clone https://github.com/TheBoatyMcBoatFace/good-pdc.git
    cd good-pdc
    ```

2. **Run locally**:

    ```sh
    cargo run
    ```

3. **Vibe Check**:

    Open up `Archives.md` and see if there are any ❌, _Hint: those are bad_

4. **Automate with GitHub Actions**:

    Push to `main` to run the bot thing. It also runs every three hours and sends notifications if there is a ❌ in the results file.

## Contributing

You're awesome for wanting to help (just saying). Here are some guidelines:

1. **Open issues**: If you find bugs or have cool ideas, open an issue. No issue = it doesn't exist.

2. **Don't be a jerk**: I am not afraid to use the ban 🔨. GitHub is the best social media platform, don't ruin it.


## License

This is aggressively open-source under [AGPL-3.0 license](https://choosealicense.com/licenses/agpl-3.0/). Details in the [LICENSE](LICENSE) file.

### Additional Links and Information

- Where the data come from: [Provider Data Catalog (PDC) API Docs](https://data.cms.gov/provider-data/docs). _Yes, data are plural. That wasn't a typo._
- **Sentry** for error tracking and performance monitoring: [Sentry Setup Guide](https://docs.sentry.io/platforms/rust/)
- **GitHub Action** for automated checks: [GitHub Actions Documentation](https://docs.github.com/en/actions)

---
_**This README was generated with AI because I'm tired and don't want to do the documenting part. Bite me**_
