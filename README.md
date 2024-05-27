# You good, PDC? 😎
![GitHub Repo stars](https://img.shields.io/github/stars/TheBoatyMcBoatFace/good-pdc)
![GitHub forks](https://img.shields.io/github/forks/TheBoatyMcBoatFace/good-pdc)

![GitHub License](https://img.shields.io/github/license/TheBoatyMcBoatFace/good-pdc)
![Last Commit](https://img.shields.io/github/last-commit/TheBoatyMcBoatFace/good-pdc)
[![Link Checker](https://github.com/TheBoatyMcBoatFace/good-pdc/actions/workflows/link_checker.yml/badge.svg)](https://github.com/TheBoatyMcBoatFace/good-pdc/actions/workflows/link_checker.yml)

This project periodically checks various links on CMS's Provider Data Catalog (PDC) to ensure that all links are operational and all data is accessible.

**THIS IS NOT AN OFFICIAL GOVERNMENT CODEBASE**.

Check the [Archives.md](Archives.md) file to see the status summary and detailed reports of each data topic. Though we're currently compiling all data into a single report, we plan to add individual pages for each data topic in the future.

Want to see if those links are still not angry? Check out the [Archives.md](Archives.md) file.

<div align="center">
  <a href="Archives.md">
    <img src="https://img.shields.io/badge/View-Archive_Link_Check_Results-brightgreen?style=for-the-badge" alt="View Link Check Status">
  </a><br><br>
</div>

## What it do?

- **Link Validation**: We check CMS PDC links to make sure they're not ghosting 👻 us.
- **Categorized Reports**: We neatly categorize this info in `Archives.md`. _I'm really proud of how this turned out._
- **Summarized Status**: A quick glance at the top tells you if things are going smoothly or if there's trouble
- **GitHub Actions**: Automatically keeps things in check every time you push to the `main` branch. It also runs every three hours and sends notifications if there is a ❌ in the results file. 

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
2. **Dont be a jerk**: I am not afraid to use the ban 🔨. GitHub is the best social media platform, don't ruin it.

## License

This is aggressively open-source under [AGPL-3.0 license](https://choosealicense.com/licenses/agpl-3.0/). Details in the [LICENSE](LICENSE) file.
