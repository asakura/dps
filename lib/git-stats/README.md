# dps-git-stats

Per-day git commit statistics: insertions, deletions, churn, and rolling
averages — rendered with Catppuccin colours.

## Usage

```text
USAGE: dps-git-stats [OPTIONS] [REVISION]

OPTIONS:
  --since <DATE>      Limit to commits on or after DATE (ISO 8601 or git date)
  --until <DATE>      Limit to commits on or before DATE
  --path  <PATH>      Restrict diff counts to this file path / sub-directory
  --repo  <DIR>       Repository path [default: .]
  --flavour <NAME>    latte | frappe | macchiato | mocha  [default: mocha]
  -h, --help
  -V, --version
```

## Output

### Per-day table

| column    | description                                                   |
|-----------|---------------------------------------------------------------|
| date      | Calendar date (YYYY-MM-DD)                                    |
| commits   | Non-merge commit count                                        |
| +lines    | Lines inserted                                                |
| -lines    | Lines deleted                                                 |
| total     | Net line delta (positive = grew, negative = shrunk)           |
| files     | File-change events                                            |
| avg_l/c   | Average lines touched per commit                              |
| churn%    | Deletion-to-insertion ratio × 100                             |
| roll7     | Rolling 7-active-day average of the net delta                 |
| cum+      | Cumulative insertions                                         |
| cum-      | Cumulative deletions                                          |
| cum_total | Cumulative net delta                                          |

### Hour distribution

Commit counts bucketed by author's local hour (00–23), scaled bar chart.
The peak 3-hour window is highlighted.

### Analysis

- **Peak coding window** — 3-hour band with most commits.
- **Anomalous days** — days with a net delta more than 2σ from the mean.
- **Active weeks** — number of ISO calendar weeks with at least one commit.

## Building

```sh
cargo build --release -p dps-git-stats
```

The binary is written to `target/release/dps-git-stats`.
