//! Statistics computation via [`polars`].
//!
//! Takes raw per-commit data from [`crate::git`] and produces a sorted,
//! aggregated [`polars::frame::DataFrame`] with one row per active day.
//!
//! Columns produced (in this order):
//!
//! | column      | description                                      |
//! |-------------|--------------------------------------------------|
//! | `date`      | Calendar date (YYYY-MM-DD)                       |
//! | `commits`   | Non-merge commit count                           |
//! | `ins`       | Lines inserted                                   |
//! | `del`       | Lines deleted                                    |
//! | `total`     | Net line delta (`ins - del`)                     |
//! | `files`     | File-change events                               |
//! | `avg_lc`    | Avg lines touched per commit                     |
//! | `churn_pct` | Deletion-to-insertion ratio × 100                |
//! | `roll7`     | Rolling 7-active-day average of `total`          |
//! | `cum_ins`   | Cumulative insertions                            |
//! | `cum_del`   | Cumulative deletions                             |
//! | `cum_total` | Cumulative net delta                             |
//!
//! ```
//! use dps_git_stats::stats::build_frame;
//! let df = build_frame(&[]).unwrap();
//! assert_eq!(df.height(), 0);
//! ```

// polars re-exports a web of interconnected traits (.lazy(), Series::new, …);
// the glob is the idiomatic import style and avoids fragile per-trait listings.
use polars::prelude::*;

use crate::error::{Error, StatsError};
use crate::git::CommitStat;

/// Rolling-window width used by [`build_frame`] and documented in the `roll7` column.
const ROLLING_WINDOW_DAYS: usize = 7;

/// Peak-hour window width shared with [`crate::render`].
pub const PEAK_WINDOW: u32 = 3;

/// Column names in display order — shared by [`build_frame`] and [`empty_frame`].
const COLUMN_ORDER: &[&str] = [
    "date",
    "commits",
    "ins",
    "del",
    "total",
    "files",
    "avg_lc",
    "churn_pct",
    "roll7",
    "cum_ins",
    "cum_del",
    "cum_total",
]
.as_slice();

/// Build the per-day aggregated [`DataFrame`] from raw commit statistics.
///
/// # Errors
///
/// Returns [`Error::Stats`] on any Polars error.
///
/// # Examples
///
/// ```
/// use dps_git_stats::git::CommitStat;
/// use dps_git_stats::stats::build_frame;
///
/// let stats = vec![CommitStat {
///     date: "2024-01-15".into(),
///     hour: 10,
///     insertions: 120,
///     deletions: 30,
///     files_changed: 5,
/// }];
/// let df = build_frame(&stats).unwrap();
/// assert_eq!(df.height(), 1);
/// ```
pub fn build_frame(stats: &[CommitStat]) -> Result<DataFrame, Error> {
    if stats.is_empty() {
        return empty_frame();
    }

    // Single pass over stats to build the four input columns.
    let capacity = stats.len();
    let mut dates = Vec::with_capacity(capacity);
    let mut ins = Vec::<u32>::with_capacity(capacity);
    let mut del = Vec::<u32>::with_capacity(capacity);
    let mut files = Vec::<u32>::with_capacity(capacity);

    for s in stats {
        dates.push(s.date.as_str());
        ins.push(s.insertions);
        del.push(s.deletions);
        files.push(s.files_changed);
    }

    let raw = df!(
        "date"  => &dates,
        "ins"   => &ins,
        "del"   => &del,
        "files" => &files,
    )
    .map_err(StatsError::Polars)?;

    // Aggregate by date.
    let by_day = raw
        .lazy()
        .group_by([col("date")])
        .agg([
            col("ins").sum().alias("ins"),
            col("del").sum().alias("del"),
            col("files").sum().alias("files"),
            col("ins").count().alias("commits"),
        ])
        .sort(["date"], SortMultipleOptions::default())
        .collect()
        .map_err(StatsError::Polars)?;

    // Derived columns — cast to Int64 before subtraction to avoid u32 underflow.
    let with_derived = by_day
        .lazy()
        .with_columns([
            (col("ins").cast(DataType::Int64) - col("del").cast(DataType::Int64)).alias("total"),
            ((col("ins").cast(DataType::Float64) + col("del").cast(DataType::Float64))
                / col("commits").cast(DataType::Float64))
            .alias("avg_lc"),
            (col("del").cast(DataType::Float64) * lit(100.0_f64)
                / col("ins")
                    .cast(DataType::Float64)
                    .clip(lit(1.0_f64), lit(f64::MAX)))
            .alias("churn_pct"),
        ])
        .collect()
        .map_err(StatsError::Polars)?;

    // Rolling active-day average (over active days, not calendar days).
    let roll7 = rolling_mean_active_days(&with_derived, "total", ROLLING_WINDOW_DAYS)?;

    // Cumulative columns — Int64 to avoid overflow on large repos.
    let mut with_cum = with_derived
        .lazy()
        .with_columns([
            col("ins")
                .cast(DataType::Int64)
                .cum_sum(false)
                .alias("cum_ins"),
            col("del")
                .cast(DataType::Int64)
                .cum_sum(false)
                .alias("cum_del"),
            col("total")
                .cast(DataType::Int64)
                .cum_sum(false)
                .alias("cum_total"),
        ])
        .collect()
        .map_err(StatsError::Polars)?;

    with_cum
        .with_column(Series::new("roll7".into(), roll7).into())
        .map_err(StatsError::Polars)?;

    // .copied() turns &&str → &str, which polars select requires.
    Ok(with_cum
        .select(COLUMN_ORDER.iter().copied())
        .map_err(StatsError::Polars)?)
}

/// Build per-hour commit counts as `Vec<(hour, count)>` sorted by hour.
///
/// Uses a simple 24-slot counter — no Polars involved.  Only hours with at
/// least one commit are included in the result.
///
/// # Examples
///
/// ```
/// use dps_git_stats::stats::hour_distribution;
/// assert!(hour_distribution(&[]).is_empty());
/// ```
pub fn hour_distribution(stats: &[CommitStat]) -> Vec<(u32, u32)> {
    let mut slots = [0u32; 24];

    for s in stats {
        slots[usize::from(s.hour)] += 1;
    }

    slots
        .into_iter()
        .enumerate()
        .filter(|(_, c)| *c > 0)
        .map(|(h, c)| (h as u32, c))
        .collect()
}

/// Map a `(hour, count)` distribution into a 24-slot array.
///
/// Hours outside `0..24` are silently ignored.  Used by both
/// [`analysis::peak_hour_band`] and [`crate::render`].
pub fn hours_to_slots(dist: &[(u32, u32)]) -> [u32; 24] {
    let mut slots = [0u32; 24];

    for &(h, c) in dist {
        if (h as usize) < 24 {
            slots[h as usize] = c;
        }
    }

    slots
}

/// Compute a rolling mean over the last `window` *active* rows.
fn rolling_mean_active_days(
    df: &DataFrame,
    col_name: &str,
    window: usize,
) -> Result<Vec<f64>, Error> {
    let values: Vec<i64> = df
        .column(col_name)
        .map_err(StatsError::Polars)?
        .cast(&DataType::Int64)
        .map_err(StatsError::Polars)?
        .i64()
        .map_err(StatsError::Polars)?
        .iter()
        .map(|v| v.unwrap_or(0))
        .collect();

    Ok(values
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let start = i.saturating_sub(window - 1);
            let slice = &values[start..=i];

            slice.iter().sum::<i64>() as f64 / slice.len() as f64
        })
        .collect())
}

/// Return an empty [`DataFrame`] with the schema expected by the renderer.
fn empty_frame() -> Result<DataFrame, Error> {
    df!(
        "date"      => &[] as &[String],
        "commits"   => &[] as &[u32],
        "ins"       => &[] as &[u32],
        "del"       => &[] as &[u32],
        "total"     => &[] as &[i64],
        "files"     => &[] as &[u32],
        "avg_lc"    => &[] as &[f64],
        "churn_pct" => &[] as &[f64],
        "roll7"     => &[] as &[f64],
        "cum_ins"   => &[] as &[i64],
        "cum_del"   => &[] as &[i64],
        "cum_total" => &[] as &[i64],
    )
    .map_err(|e| Error::Stats(StatsError::Polars(e).into()))
}

/// Extension points for future statistical analysis.
///
/// Each function accepts an aggregated [`DataFrame`] (as produced by
/// [`build_frame`]) and returns a derived result.
pub mod analysis {
    use polars::prelude::*;

    use crate::error::{Error, StatsError};

    /// Identify days where `total` deviates more than `sigma_threshold`
    /// standard deviations from the mean.
    ///
    /// Returns the anomalous rows, or an empty frame when none exist.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Stats`] on any Polars error.
    ///
    /// # Examples
    ///
    /// ```
    /// use polars::prelude::DataFrame;
    /// use dps_git_stats::stats::analysis::anomalous_days;
    ///
    /// let df = DataFrame::default();
    /// assert_eq!(anomalous_days(&df, 2.0).unwrap().height(), 0);
    /// ```
    pub fn anomalous_days(df: &DataFrame, sigma_threshold: f64) -> Result<DataFrame, Error> {
        if df.height() == 0 {
            return Ok(df.clone());
        }

        let total = df
            .column("total")
            .map_err(StatsError::Polars)?
            .cast(&DataType::Float64)
            .map_err(StatsError::Polars)?;
        let series = total.as_materialized_series();
        let mean = series.mean().unwrap_or(0.0);
        let std_dev = series.std(1).unwrap_or(0.0);
        let threshold = std_dev * sigma_threshold;

        let mask: BooleanChunked = series
            .f64()
            .map_err(StatsError::Polars)?
            .iter()
            .map(|v| Some(v.is_some_and(|x| (x - mean).abs() > threshold)))
            .collect();

        df.filter(&mask)
            .map_err(|e| Error::Stats(StatsError::Polars(e).into()))
    }

    /// Compute weekly productivity summary grouped by ISO week number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Stats`] on any Polars error.
    ///
    /// # Examples
    ///
    /// ```
    /// use polars::prelude::DataFrame;
    /// use dps_git_stats::stats::analysis::weekly_summary;
    ///
    /// assert_eq!(weekly_summary(&DataFrame::default()).unwrap().height(), 0);
    /// ```
    pub fn weekly_summary(df: &DataFrame) -> Result<DataFrame, Error> {
        if df.height() == 0 {
            return Ok(df.clone());
        }

        df.clone()
            .lazy()
            .with_column(col("date").cast(DataType::Date).dt().week().alias("week"))
            .group_by([col("week")])
            .agg([
                col("commits").sum(),
                col("ins").sum(),
                col("del").sum(),
                col("total").sum(),
            ])
            .sort(["week"], SortMultipleOptions::default())
            .collect()
            .map_err(|e| Error::Stats(StatsError::Polars(e).into()))
    }

    /// Identify the most productive hour band (peak 3-hour window by commit
    /// count).
    ///
    /// Returns `(start_hour, total_count)` for the 3-hour window with the
    /// highest commit count, or `None` for an empty distribution.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps_git_stats::stats::analysis::peak_hour_band;
    ///
    /// assert_eq!(peak_hour_band(&[]), None);
    ///
    /// let dist = [(10u32, 5u32), (11, 8), (12, 3)];
    /// assert_eq!(peak_hour_band(&dist).unwrap().0, 10);
    /// ```
    #[must_use]
    pub fn peak_hour_band(dist: &[(u32, u32)]) -> Option<(u32, u32)> {
        if dist.is_empty() {
            return None;
        }

        let slots = super::hours_to_slots(dist);

        (0u32..24)
            .map(|h| {
                let sum = (0..super::PEAK_WINDOW as usize)
                    .map(|offset| slots[(h as usize + offset) % 24])
                    .sum::<u32>();
                (h, sum)
            })
            .max_by_key(|&(_, s)| s)
    }
}

#[cfg(test)]
mod tests {
    use super::analysis::peak_hour_band;
    use super::{build_frame, hour_distribution};
    use crate::git::CommitStat;

    use polars::prelude::DataType;
    use rstest::rstest;

    fn make_stat(date: &str, hour: u8, ins: u32, del: u32, files: u32) -> CommitStat {
        CommitStat {
            date: date.into(),
            hour,
            insertions: ins,
            deletions: del,
            files_changed: files,
        }
    }

    mod build_frame {
        use super::*;

        #[rstest]
        fn empty_input() -> Result<(), &'static str> {
            let df = build_frame(&[]).map_err(|_| "build_frame failed on empty input")?;

            assert_eq!(df.height(), 0);

            Ok(())
        }

        #[rstest]
        fn empty_has_correct_columns() -> Result<(), &'static str> {
            let df = build_frame(&[]).map_err(|_| "build_frame failed")?;
            let names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();

            assert_eq!(
                names,
                [
                    "date",
                    "commits",
                    "ins",
                    "del",
                    "total",
                    "files",
                    "avg_lc",
                    "churn_pct",
                    "roll7",
                    "cum_ins",
                    "cum_del",
                    "cum_total"
                ]
            );

            Ok(())
        }

        #[rstest]
        fn single_commit() -> Result<(), &'static str> {
            let df = build_frame(&[make_stat("2024-01-15", 10, 120, 30, 5)])
                .map_err(|_| "build_frame failed")?;

            assert_eq!(df.height(), 1);

            Ok(())
        }

        #[rstest]
        fn two_commits_same_day_aggregated() -> Result<(), &'static str> {
            let stats = vec![
                make_stat("2024-01-15", 9, 50, 10, 2),
                make_stat("2024-01-15", 14, 70, 20, 3),
            ];
            let df = build_frame(&stats).map_err(|_| "build_frame failed")?;

            assert_eq!(
                df.height(),
                1,
                "two commits on same day must aggregate to 1 row"
            );

            Ok(())
        }

        #[rstest]
        fn two_different_days_sorted() -> Result<(), &'static str> {
            let stats = vec![
                make_stat("2024-01-16", 9, 50, 10, 2),
                make_stat("2024-01-15", 14, 70, 20, 3),
            ];
            let df = build_frame(&stats).map_err(|_| "build_frame failed")?;
            let first = df
                .column("date")
                .map_err(|_| "no date col")?
                .str()
                .map_err(|_| "not str")?
                .get(0)
                .ok_or("no row 0")?;

            assert_eq!(df.height(), 2);
            assert_eq!(first, "2024-01-15", "rows must be sorted ascending");

            Ok(())
        }

        #[rstest]
        fn del_exceeds_ins_gives_negative_total() -> Result<(), &'static str> {
            let df = build_frame(&[make_stat("2024-01-15", 10, 10, 50, 1)])
                .map_err(|_| "build_frame failed")?;
            let total = df
                .column("total")
                .map_err(|_| "no total")?
                .cast(&DataType::Int64)
                .map_err(|_| "cast")?
                .i64()
                .map_err(|_| "not i64")?
                .get(0)
                .ok_or("no row 0")?;

            assert_eq!(total, -40);

            Ok(())
        }
    }

    mod hour_distribution {
        use super::*;

        #[rstest]
        fn empty_input() {
            assert!(hour_distribution(&[]).is_empty());
        }

        #[rstest]
        fn single_hour_counted() {
            let stats = vec![
                make_stat("2024-01-15", 10, 1, 0, 1),
                make_stat("2024-01-15", 10, 1, 0, 1),
            ];
            let dist = hour_distribution(&stats);

            assert_eq!(dist.len(), 1);
            assert_eq!(dist[0], (10, 2));
        }

        #[rstest]
        fn multiple_hours_sorted() {
            let stats = vec![
                make_stat("2024-01-15", 14, 1, 0, 1),
                make_stat("2024-01-15", 9, 1, 0, 1),
                make_stat("2024-01-15", 14, 1, 0, 1),
            ];
            let dist = hour_distribution(&stats);

            assert_eq!(dist, [(9, 1), (14, 2)]);
        }

        #[rstest]
        fn zero_hours_omitted() {
            let stats = vec![make_stat("2024-01-15", 5, 1, 0, 1)];
            let dist = hour_distribution(&stats);

            assert_eq!(dist.len(), 1);
            assert_eq!(dist[0].0, 5);
        }
    }

    mod peak_hour_band {
        use super::*;

        #[rstest]
        fn empty() {
            assert_eq!(peak_hour_band(&[]), None);
        }

        #[rstest]
        fn single_entry() {
            assert_eq!(peak_hour_band(&[(10, 5)]), Some((10, 5)));
        }

        #[rstest]
        fn best_window_selected() -> Result<(), &'static str> {
            let dist = [(10u32, 5u32), (11, 8), (12, 3), (15, 1)];

            assert_eq!(peak_hour_band(&dist).ok_or("no peak hour found")?.0, 10);

            Ok(())
        }
    }
}
