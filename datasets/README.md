# Dataset Link Check Results 📊

![GitHub Repo stars](https://img.shields.io/github/stars/TheBoatyMcBoatFace/good-pdc)
![GitHub forks](https://img.shields.io/github/forks/TheBoatyMcBoatFace/good-pdc)

![GitHub License](https://img.shields.io/github/license/TheBoatyMcBoatFace/good-pdc)
![Last Commit](https://img.shields.io/github/last-commit/TheBoatyMcBoatFace/good-pdc)
[![Link Checker](https://github.com/TheBoatyMcBoatFace/good-pdc/actions/workflows/link_checker.yml/badge.svg)](https://github.com/TheBoatyMcBoatFace/good-pdc/actions/workflows/link_checker.yml)

This section contains detailed link check results for various datasets on the CMS's Provider Data Catalog (PDC). Each dataset has its own report detailing the status of the links and the accessibility of the data.

## Available Datasets

Here are the datasets we currently monitor:

- **[DAC](DAC.md)**: Doctors and Clinicians
- **[DF](DF.md)**: Dialysis Facilities
- **[HC](HC.md)**: Hospice Care
- **[HHS](HHS.md)**: Home Health Services
- **[HOS](HOS.md)**: Hospitals
- **[IRF](IRF.md)**: Inpatient Rehabilitation Facilities
- **[LTCH](LTCH.md)**: Long-Term Care Hospitals
- **[NH](NH.md)**: Nursing Homes Including Rehab Services
- **[PPL](PPL.md)**: Physician Office Visit Costs
- **[SUP](SUP.md)**: Supplier Directory

## Template for Dataset Reports

Each dataset report follows a consistent template to provide a comprehensive overview of the dataset's status and details. Below is a description of the sections included in each dataset markdown file:

### Dataset Report Structure

1. **Dataset Title**
   - Brief description of the dataset and its scope.
   - **Dataset ID:** Unique identifier for the dataset.
   - **Status:** Current status of the dataset (e.g., ✅ for accessible, ❌ for issues).

2. **Dataset Details**
   - **File History**: Detailed history of the dataset file, including creation, modification, release, and last checked dates.
     <details>
     <summary>File History</summary>
     <table>
       <tr><th>Activity</th><th>Description</th><th>Date</th></tr>
       <tr><td>Issued Date</td><td>When the dataset was created</td><td>YYYY-MM-DD</td></tr>
       <tr><td>Modified Date</td><td>When it was last modified</td><td>YYYY-MM-DD</td></tr>
       <tr><td>Release Date</td><td>When the dataset was made public</td><td>YYYY-MM-DD</td></tr>
       <tr><td>Last Checked</td><td>When this dataset was last tested</td><td>YYYY-MM-DD</td></tr>
     </table>
     </details>

   - **File Overview**: Metrics related to the dataset file, such as filesize, row count, and column count.
     <details>
     <summary>File Overview</summary>
     <table>
       <tr><th>Metric</th><th>Result</th></tr>
       <tr><td>Filesize</td><td>0.0 MB</td></tr>
       <tr><td>Row Count</td><td>55</td></tr>
       <tr><td>Column Count</td><td>8</td></tr>
     </table>
     </details>

3. **Data Integrity Tests**
   - Summary and results of basic data integrity tests, including column count consistency, header validation, and encoding validation.
     <details>
     <summary>✅ </summary>
     <table>
       <tr><th>Test</th><th>Description</th><th>Result</th></tr>
       <tr><td>Column Count Consistency</td><td>Verify that all rows have the same number of columns.</td><td>✅</td></tr>
       <tr><td>Header Validation</td><td>Ensure the CSV has a header row and all headers are unique and meaningful.</td><td>✅</td></tr>
       <tr><td>Encoding Validation</td><td>Verify that the CSV file uses UTF-8 encoding.</td><td>UTF-8</td></tr>
     </table>
     </details>

4. **Public Access Tests**
   - Tests for public accessibility and A11y (Accessibility) compliance for dataset resources.
     <table>
       <tr><th>Page</th><th>Status</th><th>A11y Test</th></tr>
       <tr><td>[PDC Page](#)</td><td>✅</td><td>[![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl=#)](#)</td></tr>
       <tr><td>[Landing Page](#)</td><td>✅</td><td>[![W3C Validation](https://img.shields.io/w3c-validation/default?targetUrl=#)](#)</td></tr>
       <tr><td>[Direct Download](#)</td><td>✅</td><td></td></tr>
     </table>

## Automated Checks

Our GitHub Actions workflow automatically runs these link checks every three hours and sends notifications if any issues are detected. You can view the latest workflow run results by clicking the badge above.

## Contributing

If you notice any issues or have suggestions for additional datasets to monitor, please open an issue or submit a pull request. We appreciate your contributions!

<div align="center">
  <a href="..">
    <img src="https://img.shields.io/badge/Back_to_Main-Repository-7B68EE?style=for-the-badge" alt="Back to Main Repository">
  </a>
</div>

## How the Dataset Reports are Generated

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

This ensures that all datasets listed on the Provider Data Catalog are regularly tested for accessibility and data integrity, with results being documented in a consistent and transparent manner.

For more details on the implementation, refer to the [source code](../src/dataset_report.rs).

---
_**This README was generated with AI because I'm tired and don't want to do the documenting part. Bite me**_
