#!/usr/bin/env bash
# Commits per day with total lines added/removed.
# Usage: git-stats-per-day.sh [--since DATE] [--until DATE] [BRANCH]

GIT_ARGS=(--no-merges --shortstat '--format=COMMIT:%ad' '--date=format:%Y-%m-%d|%H')

while [[ $# -gt 0 ]]; do
    case $1 in
        --since) GIT_ARGS+=(--since="$2"); shift 2 ;;
        --until) GIT_ARGS+=(--until="$2"); shift 2 ;;
        --help|-h)
            cat <<'EOF'
Usage: git-stats-per-day.sh [--since DATE] [--until DATE] [BRANCH]

Prints a per-day activity table followed by an hour distribution chart.
Merge commits are excluded.

OPTIONS
  --since DATE   Limit to commits on or after DATE (passed to git log --since)
  --until DATE   Limit to commits on or before DATE (passed to git log --until)
  BRANCH         Any extra argument is forwarded to git log (branch, range, path…)
  --help, -h     Show this help and exit

TABLE COLUMNS
  date       Calendar date (YYYY-MM-DD).

  commits    Number of non-merge commits on that day.

  +lines     Lines inserted across all commits.

  -lines     Lines deleted across all commits.

  total      Net line delta for the day (+lines minus -lines). Positive means
             the codebase grew; negative means more was cut than written.

  files      Number of file-change events (one file touched in one commit counts
             once; the same file in a second commit counts again). Gives a sense
             of breadth: touching 40 files vs. 3 is very different work.

  avg_l/c    Average lines touched per commit ((+lines + -lines) / commits).
             Small values mean focused, granular commits. A rising average is
             often a sign of fatigue or rushed work where changes pile up before
             being committed.

  churn%     Deletion-to-insertion ratio expressed as a percentage
             (-lines / +lines × 100). Near 0% = mostly new code; near 100% =
             heavy rewriting with little net growth; above 100% = codebase
             shrank. Useful for spotting refactoring days vs. feature days.

  roll7      Rolling 7-active-day average of the net line delta. Smooths out
             single-day spikes so the underlying velocity trend is visible.
             "Active days" means days that had at least one commit; calendar
             gaps (weekends, breaks) are skipped.

  cum+       Cumulative lines inserted since the first commit in the range.

  cum-       Cumulative lines deleted since the first commit in the range.

  cum_total  Cumulative net line delta (cum+ minus cum-). Shows the overall
             growth trajectory of the codebase over time.

HOUR DISTRIBUTION
  Commit counts bucketed by the author's local hour of day (00–23). The bar
  width is scaled to the busiest hour. Useful for identifying your productive
  window and for checking whether late-night commits tend to be larger or
  messier than daytime ones (cross-reference avg_l/c and churn% on heavy
  evening days).
EOF
            exit 0
            ;;
        *)       GIT_ARGS+=("$1"); shift ;;
    esac
done

git log "${GIT_ARGS[@]}" | gawk '
/^COMMIT:/ {
    split(substr($0, 8), parts, "|")
    date = parts[1]
    hour = parts[2]
    commits[date]++
    hour_count[hour]++
}
/files? changed/ {
    ins = 0; del = 0; fls = 0
    for (i = 1; i <= NF; i++) {
        if ($i ~ /insertion/) ins = $(i-1)
        if ($i ~ /deletion/)  del = $(i-1)
        if ($i ~ /^files?$/)   fls = $(i-1)
    }
    added[date]   += ins
    removed[date] += del
    files[date]   += fls
}
END {
    n = asorti(commits, dates)

    for (i = 1; i <= n; i++) {
        cnt = (i < 7) ? i : 7
        sum = 0
        for (j = i - cnt + 1; j <= i; j++)
            sum += added[dates[j]] - removed[dates[j]]
        roll7[dates[i]] = sum / cnt
    }

    printf "%-12s %7s %8s %8s %8s %6s %8s %7s %8s %10s %10s %10s\n", \
        "date", "commits", "+lines", "-lines", "total", "files", "avg_l/c", "churn%", "roll7", "cum+", "cum-", "cum_total"
    printf "%-12s %7s %8s %8s %8s %6s %8s %7s %8s %10s %10s %10s\n", \
        "----", "-------", "------", "------", "-----", "-----", "-------", "------", "-----", "----", "----", "---------"

    cum_add = 0; cum_rem = 0
    for (i = 1; i <= n; i++) {
        d = dates[i]
        total  = added[d] - removed[d]
        avg_lc = commits[d] > 0 ? int((added[d] + removed[d]) / commits[d]) : 0
        churn  = added[d]   > 0 ? int(removed[d] * 100 / added[d])          : 0
        cum_add += added[d]
        cum_rem += removed[d]
        printf "%-12s %7d %8d %8d %+8d %6d %8d %6d%% %+8d %10d %10d %+10d\n", \
            d, commits[d], added[d], removed[d], total, \
            files[d], avg_lc, churn, roll7[d], \
            cum_add, cum_rem, cum_add - cum_rem
    }

    printf "\nCommit hour distribution (author local time):\n"
    max_h = 0
    for (h in hour_count)
        if (hour_count[h] > max_h) max_h = hour_count[h]
    for (h = 0; h <= 23; h++) {
        hh = sprintf("%02d", h)
        cnt = (hh in hour_count) ? hour_count[hh] : 0
        bar_len = (max_h > 0) ? int(cnt * 40 / max_h) : 0
        bar = ""
        for (k = 0; k < bar_len; k++) bar = bar "#"
        printf "  %s  %-40s %d\n", hh, bar, cnt
    }
}'
