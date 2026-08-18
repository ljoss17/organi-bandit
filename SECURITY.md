# Security Policy

Organi-Bandit is a local desktop application. It doesn't run a server, doesn't accept input over a network, and only reads or writes files where you point it (team lists, the exported spreadsheet). Most security concerns typical of a web application don't apply here.

That said, if you find a genuine security issue, for example a vulnerable dependency, a crafted input file that causes a crash or unexpected behavior, or unsafe handling of file paths, please report it privately rather than opening a public issue.

## Reporting

Preferred: use GitHub's [private vulnerability reporting](https://github.com/ljoss17/organi-bandit/security/advisories/new) for this repository.

If that isn't available, email [43531661+ljoss17@users.noreply.github.com](mailto:43531661+ljoss17@users.noreply.github.com) with details and, if possible, steps to reproduce.

This is a spare-time project, so response times will vary, but reports won't be ignored.

## Scope

In scope: the Rust backend, the Tauri command layer, and dependency vulnerabilities affecting them.

Out of scope: general bugs with no security impact (use a regular issue instead), and social engineering or physical access scenarios, since the app only ever operates on the local machine it's installed on.
