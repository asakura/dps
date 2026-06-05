//! Command-line argument definitions for `dps-git-stats`.
//!
//! Parsed via [`clap`] with the derive API.  All fields map 1-to-1 to the
//! original shell-script flags plus additional quality-of-life options.
//!
//! # Examples
//!
//! ```
//! use dps_git_stats::args::Args;
//! use clap::Parser;
//!
//! // Minimal invocation — no filters, current directory.
//! let args = Args::try_parse_from(["dps-git-stats"]).unwrap();
//! assert!(args.since.is_none());
//! assert!(args.until.is_none());
//! assert!(args.path.is_none());
//! assert!(args.revision.is_none());
//! ```

use std::path::PathBuf;

use clap::Parser;

/// Per-day git commit statistics.
///
/// Prints a per-day activity table followed by an hour distribution chart.
/// Merge commits are always excluded.
///
/// All options with dates accept any format that `git log` accepts, including
/// ISO 8601 (`2024-01-01`) and relative expressions (`"1 month ago"`).
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Limit to commits on or after this date.
    ///
    /// The value is compared lexicographically against the `YYYY-MM-DD`
    /// author date of each commit, so any ISO-8601 date string works.
    #[arg(long, value_name = "DATE")]
    pub since: Option<String>,

    /// Limit to commits on or before this date.
    ///
    /// Same format rules as `--since`.
    #[arg(long, value_name = "DATE")]
    pub until: Option<String>,

    /// Restrict diff line counts to files under this path or directory.
    ///
    /// The filter is applied per file change: only changes whose location
    /// starts with the given prefix are counted toward insertions/deletions.
    /// The commit still appears in the table if at least one change passes
    /// the filter.
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Git revision, branch, or range to walk from.
    ///
    /// Any rev-spec understood by `git rev-parse` is accepted:
    /// `main`, `HEAD~50`, `v1.0.0`, etc.  Defaults to `HEAD`.
    #[arg(value_name = "REVISION")]
    pub revision: Option<String>,

    /// Path to the git repository.
    ///
    /// May point to either the working tree root or the bare `.git` directory.
    /// Defaults to the current working directory.
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub repo: PathBuf,

    /// Catppuccin colour flavour for terminal output.
    ///
    /// All four Catppuccin flavours are supported.  On non-TTY outputs (pipes,
    /// redirects) ANSI codes are automatically stripped by [`anstream`].
    #[arg(long, value_name = "FLAVOUR", default_value = "mocha")]
    pub flavour: Flavour,
}

/// Catppuccin colour flavour selector.
///
/// Controls which of the four Catppuccin palettes is used when rendering
/// colours to the terminal.
///
/// # Examples
///
/// ```
/// use dps_git_stats::args::Flavour;
/// use clap::Parser;
/// use dps_git_stats::args::Args;
///
/// let args = Args::try_parse_from(["dps-git-stats", "--flavour", "latte"]).unwrap();
/// assert_eq!(args.flavour, Flavour::Latte);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Flavour {
    /// Catppuccin Latte — the light theme.
    Latte,
    /// Catppuccin Frappé — a soft dark theme.
    Frappe,
    /// Catppuccin Macchiato — a medium dark theme.
    Macchiato,
    /// Catppuccin Mocha — the darkest theme (default).
    Mocha,
}

impl From<Flavour> for catppuccin::Flavor {
    /// Convert a CLI [`Flavour`] selector into the corresponding
    /// [`catppuccin::Flavor`] palette value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps_git_stats::args::Flavour;
    ///
    /// let palette = catppuccin::Flavor::from(Flavour::Mocha);
    /// assert_eq!(palette.name, catppuccin::FlavorName::Mocha);
    /// ```
    fn from(f: Flavour) -> Self {
        match f {
            Flavour::Latte => catppuccin::PALETTE.latte,
            Flavour::Frappe => catppuccin::PALETTE.frappe,
            Flavour::Macchiato => catppuccin::PALETTE.macchiato,
            Flavour::Mocha => catppuccin::PALETTE.mocha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Args, Flavour};

    use clap::Parser;
    use rstest::rstest;

    mod args {
        use super::*;

        #[rstest]
        fn defaults() -> Result<(), &'static str> {
            let args = Args::try_parse_from(["dps-git-stats"]).map_err(|_| "parse failed")?;

            assert!(args.since.is_none());
            assert!(args.until.is_none());
            assert!(args.path.is_none());
            assert!(args.revision.is_none());
            assert_eq!(args.flavour, Flavour::Mocha);

            Ok(())
        }

        #[rstest]
        fn since_and_until() -> Result<(), &'static str> {
            let args = Args::try_parse_from([
                "dps-git-stats",
                "--since",
                "2024-01-01",
                "--until",
                "2024-12-31",
            ])
            .map_err(|_| "parse failed")?;

            assert_eq!(args.since.as_deref(), Some("2024-01-01"));
            assert_eq!(args.until.as_deref(), Some("2024-12-31"));

            Ok(())
        }

        #[rstest]
        fn revision_positional() -> Result<(), &'static str> {
            let args =
                Args::try_parse_from(["dps-git-stats", "main"]).map_err(|_| "parse failed")?;

            assert_eq!(args.revision.as_deref(), Some("main"));

            Ok(())
        }
    }

    mod flavour {
        use super::*;

        #[rstest]
        #[case("latte", Flavour::Latte)]
        #[case("frappe", Flavour::Frappe)]
        #[case("macchiato", Flavour::Macchiato)]
        #[case("mocha", Flavour::Mocha)]
        fn all_flavours_parse(
            #[case] name: &str,
            #[case] expected: Flavour,
        ) -> Result<(), &'static str> {
            let args = Args::try_parse_from(["dps-git-stats", "--flavour", name])
                .map_err(|_| "parse failed")?;

            assert_eq!(args.flavour, expected);

            Ok(())
        }

        #[rstest]
        fn from_mocha_gives_mocha_palette() {
            let flavor = catppuccin::Flavor::from(Flavour::Mocha);

            assert_eq!(flavor.name, catppuccin::FlavorName::Mocha);
        }

        #[rstest]
        fn from_latte_gives_latte_palette() {
            let flavor = catppuccin::Flavor::from(Flavour::Latte);

            assert_eq!(flavor.name, catppuccin::FlavorName::Latte);
        }
    }
}
