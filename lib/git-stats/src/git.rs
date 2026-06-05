//! Git repository traversal using [`gix`].
//!
//! Walks the commit graph from the given revision (defaulting to `HEAD`),
//! skipping merge commits, and collects per-commit statistics: author date,
//! hour, line insertions, line deletions, and files changed.
//!
//! ```no_run
//! use dps_git_stats::args::Args;
//! use dps_git_stats::git::collect_stats;
//! use clap::Parser;
//!
//! let args = Args::try_parse_from(["dps-git-stats"]).unwrap();
//! let stats = collect_stats(&args).unwrap();
//! assert!(stats.iter().all(|s| s.files_changed > 0 || s.insertions == 0));
//! ```

use gix::bstr::ByteSlice;
use gix::diff::blob;
use gix::object::tree::diff::ChangeDetached;
use gix::revision::walk::Sorting;
use gix::traverse::commit::simple::CommitTimeOrder;

use crate::args::Args;
use crate::error::{Error, GitError};

const SECS_PER_DAY: i64 = 86_400;
const SECS_PER_HOUR: i64 = 3_600;

/// Raw per-commit statistics extracted from the repository.
///
/// One value is produced for each non-merge commit that passes the date and
/// path filters in [`collect_stats`].  The diff counts reflect only the files
/// that match `args.path` (all files when `args.path` is `None`).
///
/// # Examples
///
/// ```
/// use dps_git_stats::git::CommitStat;
///
/// let s = CommitStat {
///     date: "2024-06-01".into(),
///     hour: 14,
///     insertions: 42,
///     deletions: 7,
///     files_changed: 3,
/// };
/// assert_eq!(s.insertions - s.deletions, 35);
/// ```
#[derive(Debug, Clone)]
pub struct CommitStat {
    /// Author date in `YYYY-MM-DD` format.
    pub date: String,
    /// Author local hour of day (`0`–`23`).
    pub hour: u8,
    /// Lines inserted by this commit.
    pub insertions: u32,
    /// Lines deleted by this commit.
    pub deletions: u32,
    /// Number of files changed by this commit.
    pub files_changed: u32,
}

/// Collect per-commit statistics from the repository described by `args`.
///
/// Merge commits (more than one parent) are skipped.  If `args.revision` is
/// set the walk starts from that rev-spec; otherwise it starts from `HEAD`.
/// If `args.path` is set only file changes under that path are counted.
///
/// # Errors
///
/// Returns [`Error::Git`] if the repository cannot be opened, the revision
/// cannot be resolved, or the commit graph cannot be walked.
///
/// # Examples
///
/// ```no_run
/// use dps_git_stats::args::Args;
/// use dps_git_stats::git::collect_stats;
/// use clap::Parser;
///
/// let args = Args::try_parse_from(["dps-git-stats", "--since", "2024-01-01"]).unwrap();
/// let stats = collect_stats(&args).unwrap();
///
/// for s in &stats {
///     assert!(s.date >= "2024-01-01");
/// }
/// ```
pub fn collect_stats(args: &Args) -> Result<Vec<CommitStat>, Error> {
    let repo = gix::open(&args.repo).map_err(GitError::Open)?;
    let tip = resolve_tip(&repo, args)?;

    // Convert the path filter once, outside the commit loop.
    let path_filter: Option<String> = args.path.as_ref().map(|p| p.to_string_lossy().into_owned());

    let walk = repo
        .rev_walk([tip])
        .sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .all()
        .map_err(GitError::Walk)?;

    let mut stats = Vec::new();

    for info in walk {
        let info = info.map_err(GitError::WalkIter)?;
        let commit = repo
            .find_object(info.id)
            .map_err(GitError::Object)?
            .try_into_commit()
            .map_err(|e| GitError::ObjectKind(e.to_string()))?;

        // Collect parent IDs in one pass to avoid decoding the commit twice.
        let parent_ids: Vec<gix::ObjectId> = commit.parent_ids().map(gix::Id::detach).collect();

        if parent_ids.len() > 1 {
            continue; // skip merge commits
        }

        let author = commit.author().map_err(GitError::Decode)?;
        let (date, hour) = format_time(author.seconds());

        if !date_in_range(&date, args.since.as_deref(), args.until.as_deref()) {
            continue;
        }

        let new_tree = commit.tree().map_err(|e| GitError::Tree(e.to_string()))?;

        let old_tree = parent_ids
            .first()
            .copied()
            .map(|pid| -> Result<gix::Tree<'_>, Error> {
                let tree = repo
                    .find_object(pid)
                    .map_err(GitError::Object)?
                    .try_into_commit()
                    .map_err(|e| GitError::ObjectKind(e.to_string()))?
                    .tree()
                    .map_err(|e| GitError::Tree(e.to_string()))?;
                Ok(tree)
            })
            .transpose()?;

        let changes = repo
            .diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), None)
            .map_err(|e| GitError::TreeDiff(e.to_string()))?;

        let (files_changed, insertions, deletions) =
            count_changes(&repo, &changes, path_filter.as_deref())?;

        stats.push(CommitStat {
            date,
            hour,
            insertions,
            deletions,
            files_changed,
        });
    }

    Ok(stats)
}

/// Resolve the start revision to an [`gix::ObjectId`].
///
/// Uses `args.revision` when provided; falls back to `HEAD`.
///
/// # Errors
///
/// Returns [`Error::Git`] wrapping either [`GitError::RevSpec`] or
/// [`GitError::Head`].
///
/// # Examples
///
/// ```no_run
/// use dps_git_stats::args::Args;
/// use dps_git_stats::git::collect_stats;
/// use clap::Parser;
///
/// // Using a branch name as the revision.
/// let args = Args::try_parse_from(["dps-git-stats", "main"]).unwrap();
/// let stats = collect_stats(&args).unwrap();
/// ```
fn resolve_tip(repo: &gix::Repository, args: &Args) -> Result<gix::ObjectId, Error> {
    Ok(match &args.revision {
        Some(rev) => repo
            .rev_parse_single(rev.as_str())
            .map_err(|e| GitError::RevSpec(e.to_string()))?
            .detach(),
        None => repo.head_commit().map_err(GitError::Head)?.id,
    })
}

/// Aggregate `(files_changed, insertions, deletions)` across all changes,
/// filtering to paths that start with `path_prefix` when provided.
fn count_changes(
    repo: &gix::Repository,
    changes: &[gix::object::tree::diff::ChangeDetached],
    path_prefix: Option<&str>,
) -> Result<(u32, u32, u32), Error> {
    let mut files_changed: u32 = 0;
    let mut insertions: u32 = 0;
    let mut deletions: u32 = 0;

    for change in changes {
        if path_prefix.is_some_and(|prefix| !change_location(change).starts_with_str(prefix)) {
            continue;
        }

        let (ins, del) = count_change_lines(repo, change)?;

        files_changed += 1;
        insertions += ins;
        deletions += del;
    }

    Ok((files_changed, insertions, deletions))
}

/// Extract the file path from any variant of
/// [`gix::object::tree::diff::ChangeDetached`].
fn change_location(change: &gix::object::tree::diff::ChangeDetached) -> &gix::bstr::BStr {
    match change {
        ChangeDetached::Addition { location, .. }
        | ChangeDetached::Deletion { location, .. }
        | ChangeDetached::Modification { location, .. }
        | ChangeDetached::Rewrite { location, .. } => location.as_bstr(),
    }
}

/// Count `(insertions, deletions)` for a single file change via the Myers
/// line-diff algorithm.
fn count_change_lines(
    repo: &gix::Repository,
    change: &gix::object::tree::diff::ChangeDetached,
) -> Result<(u32, u32), Error> {
    let (old_id, new_id) = match change {
        ChangeDetached::Addition { id, .. } => (None, Some(*id)),
        ChangeDetached::Deletion { id, .. } => (Some(*id), None),
        ChangeDetached::Modification {
            previous_id, id, ..
        } => (Some(*previous_id), Some(*id)),
        ChangeDetached::Rewrite { source_id, id, .. } => (Some(*source_id), Some(*id)),
    };

    let old_data: Vec<u8> = old_id
        .map(|id| repo.find_object(id).map(|o| o.data.clone()))
        .transpose()
        .map_err(GitError::Object)?
        .unwrap_or_default();

    let new_data: Vec<u8> = new_id
        .map(|id| repo.find_object(id).map(|o| o.data.clone()))
        .transpose()
        .map_err(GitError::Object)?
        .unwrap_or_default();

    let input = blob::InternedInput::new(old_data.as_slice(), new_data.as_slice());
    let diff = blob::Diff::compute(blob::Algorithm::Myers, &input);

    Ok((diff.count_additions(), diff.count_removals()))
}

/// Format a Unix timestamp (seconds since Unix epoch) into `(YYYY-MM-DD, hour)`.
///
/// The date is in the Gregorian calendar.  The hour is 0–23 in the author's
/// local time (as stored in the git commit object; no timezone adjustment).
///
/// # Examples
///
/// ```
/// use dps_git_stats::git::format_time;
///
/// assert_eq!(format_time(0), ("1970-01-01".into(), 0u8));
/// assert_eq!(format_time(1_705_328_920), ("2024-01-15".into(), 14u8));
///
/// // Last second of 1970-01-01
/// let (date, hour) = format_time(86399);
/// assert_eq!(date, "1970-01-01");
/// assert_eq!(hour, 23u8);
/// ```
pub fn format_time(unix_secs: i64) -> (String, u8) {
    let hour = (unix_secs.rem_euclid(SECS_PER_DAY) / SECS_PER_HOUR) as u8;
    let (y, m, d) = days_to_ymd((unix_secs / SECS_PER_DAY) as i32);

    (format!("{y:04}-{m:02}-{d:02}"), hour)
}

/// Convert days since Unix epoch (1970-01-01) to `(year, month, day)`.
///
/// Algorithm: <https://howardhinnant.github.io/date_algorithms.html#civil_from_days>.
const fn days_to_ymd(z: i32) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y, m, d)
}

/// Return `true` if `date` (YYYY-MM-DD) falls within the optional
/// `[since, until]` window.
///
/// Comparison is purely lexicographic, which is correct for ISO 8601 date
/// strings.  Both bounds are inclusive.  A `None` bound means unbounded.
///
/// # Examples
///
/// ```
/// use dps_git_stats::git::date_in_range;
///
/// assert!( date_in_range("2024-06-15", None, None));
/// assert!( date_in_range("2024-01-01", Some("2024-01-01"), None));
/// assert!(!date_in_range("2023-12-31", Some("2024-01-01"), None));
/// assert!( date_in_range("2024-12-31", None, Some("2024-12-31")));
/// assert!(!date_in_range("2025-01-01", None, Some("2024-12-31")));
/// assert!( date_in_range("2024-06-15", Some("2024-01-01"), Some("2024-12-31")));
/// assert!(!date_in_range("2023-01-01", Some("2024-01-01"), Some("2024-12-31")));
/// ```
pub fn date_in_range(date: &str, since: Option<&str>, until: Option<&str>) -> bool {
    if let Some(s) = since
        && date < s
    {
        return false;
    }

    if let Some(u) = until
        && date > u
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{date_in_range, days_to_ymd, format_time};

    use rstest::rstest;

    mod days_to_ymd {
        use super::*;

        #[rstest]
        #[case(0,     (1970, 1,  1))]
        #[case(1,     (1970, 1,  2))]
        #[case(365,   (1971, 1,  1))]
        #[case(18628, (2021, 1,  1))]
        fn known_dates(#[case] days: i32, #[case] expected: (i32, u32, u32)) {
            assert_eq!(days_to_ymd(days), expected);
        }

        #[rstest]
        fn epoch_minus_one() {
            assert_eq!(days_to_ymd(-1), (1969, 12, 31));
        }
    }

    mod format_time {
        use super::*;

        #[rstest]
        fn epoch() {
            assert_eq!(format_time(0), ("1970-01-01".into(), 0u8));
        }

        #[rstest]
        fn known_timestamp() {
            // 2024-01-15 14:32:00 UTC → 1705328920
            assert_eq!(format_time(1_705_328_920), ("2024-01-15".into(), 14u8));
        }

        #[rstest]
        fn midnight_boundary() {
            let (date, hour) = format_time(86399);
            assert_eq!(date, "1970-01-01");
            assert_eq!(hour, 23u8);
        }
    }

    mod date_in_range {
        use super::*;

        #[rstest]
        fn no_bounds() {
            assert!(date_in_range("2024-06-01", None, None));
        }

        #[rstest]
        fn after_since() {
            assert!(date_in_range("2024-06-01", Some("2024-01-01"), None));
        }

        #[rstest]
        fn before_since_excluded() {
            assert!(!date_in_range("2023-12-31", Some("2024-01-01"), None));
        }

        #[rstest]
        fn on_since_boundary_included() {
            assert!(date_in_range("2024-01-01", Some("2024-01-01"), None));
        }

        #[rstest]
        fn before_until() {
            assert!(date_in_range("2024-06-01", None, Some("2024-12-31")));
        }

        #[rstest]
        fn after_until_excluded() {
            assert!(!date_in_range("2025-01-01", None, Some("2024-12-31")));
        }

        #[rstest]
        fn on_until_boundary_included() {
            assert!(date_in_range("2024-12-31", None, Some("2024-12-31")));
        }

        #[rstest]
        fn within_both_bounds() {
            assert!(date_in_range(
                "2024-06-15",
                Some("2024-01-01"),
                Some("2024-12-31")
            ));
        }

        #[rstest]
        fn outside_both_bounds() {
            assert!(!date_in_range(
                "2023-01-01",
                Some("2024-01-01"),
                Some("2024-12-31")
            ));
        }
    }
}
