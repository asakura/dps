//! Command-line interface: argument parsing and version string construction.

use clap::{CommandFactory, Parser};
use directories::ProjectDirs;

use std::{env, path::PathBuf, sync::LazyLock, sync::OnceLock};

static VERSION: OnceLock<String> = OnceLock::new();

/// Upper-cased crate name, used as the prefix for environment variables
/// (`DPS_DATA`, `DPS_CONFIG`, `DPS_LOG_LEVEL`).
pub static PROJECT_NAME: LazyLock<String> =
    LazyLock::new(|| env!("CARGO_CRATE_NAME").to_uppercase());

/// Command-line arguments for `dps`.
#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Input-polling rate in Hz — how often the event loop checks for key presses.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0, value_parser = Self::parse_positive_hz)]
    pub tick_rate: f64,

    /// Render rate in Hz — maximum number of frames drawn per second.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0, value_parser = Self::parse_positive_hz)]
    pub frame_rate: f64,

    /// Override the data directory (logs, state). Defaults to the platform data dir
    /// or the `DPS_DATA` environment variable.
    #[arg(long, value_name = "PATH", default_value_os_t = Self::default_data_dir(), value_parser = Self::parse_dir)]
    pub data_dir: PathBuf,

    /// Override the config directory. Defaults to the platform config dir
    /// or the `DPS_CONFIG` environment variable.
    #[arg(long, value_name = "PATH", default_value_os_t = Self::default_config_dir(), value_parser = Self::parse_dir)]
    pub config_dir: PathBuf,
}

impl Cli {
    fn parse_positive_hz(s: &str) -> Result<f64, String> {
        let v: f64 = s
            .parse()
            .map_err(|_| format!("`{s}` is not a valid number"))?;

        if v > 0.0 && v.is_finite() {
            Ok(v)
        } else {
            Err(format!("`{s}` must be a positive finite number"))
        }
    }

    fn parse_dir(s: &str) -> Result<PathBuf, String> {
        if s.is_empty() {
            Err("path must not be empty".to_string())
        } else {
            Ok(PathBuf::from(s))
        }
    }

    fn project_directory() -> Option<ProjectDirs> {
        // TODO: what are qualifier and organization?
        ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
    }

    /// Returns the configuration directory.
    ///
    /// Resolution order:
    /// 1. `DPS_CONFIG` environment variable
    /// 2. Platform config directory (`~/.config/dps` on Linux)
    /// 3. `.config` in the current working directory
    fn default_config_dir() -> PathBuf {
        env::var(format!("{}_CONFIG", *PROJECT_NAME)).map_or_else(
            |_| {
                Self::project_directory().map_or_else(
                    || PathBuf::from(".").join(".config"),
                    |d| d.config_local_dir().to_path_buf(),
                )
            },
            PathBuf::from,
        )
    }

    /// Returns the data directory used for logs and application state.
    ///
    /// Resolution order:
    /// 1. `DPS_DATA` environment variable
    /// 2. Platform data directory (`~/.local/share/dps` on Linux)
    /// 3. `.data` in the current working directory
    fn default_data_dir() -> PathBuf {
        env::var(format!("{}_DATA", *PROJECT_NAME)).map_or_else(
            |_| {
                Self::project_directory().map_or_else(
                    || PathBuf::from(".").join(".data"),
                    |d| d.data_local_dir().to_path_buf(),
                )
            },
            PathBuf::from,
        )
    }
}

/// Validated, ready-to-use arguments extracted from [`Cli`].
///
/// Constructed via [`TryFrom<Cli>`]; the conversion enforces that
/// `frame_rate >= tick_rate`.
#[derive(Debug)]
pub struct Args {
    /// Input-polling rate in Hz.
    tick_rate: f64,
    /// Render rate in Hz.
    frame_rate: f64,
    /// Resolved data directory (logs, state).
    data_dir: PathBuf,
    /// Resolved configuration directory.
    config_dir: PathBuf,
}

impl Args {
    /// Input-polling rate in Hz.
    #[must_use]
    pub const fn tick_rate(&self) -> f64 {
        self.tick_rate
    }

    /// Render rate in Hz.
    #[must_use]
    pub const fn frame_rate(&self) -> f64 {
        self.frame_rate
    }

    /// Resolved data directory (logs, state).
    #[must_use]
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// Resolved configuration directory.
    #[must_use]
    pub fn config_dir(&self) -> &std::path::Path {
        &self.config_dir
    }
}

impl TryFrom<Cli> for Args {
    type Error = clap::error::Error;

    /// Validates cross-argument constraints after parsing.
    ///
    /// Returns an error if `--frame-rate` is less than `--tick-rate`.
    ///
    /// # Errors
    ///
    /// Returns a [`clap::error::Error`] with kind
    /// [`ArgumentConflict`](clap::error::ErrorKind::ArgumentConflict) when
    /// `frame_rate < tick_rate`.
    ///
    /// # Examples
    ///
    /// ```
    /// use clap::Parser;
    /// use dps::cli::{Args, Cli};
    ///
    /// let cli = Cli::try_parse_from(["dps", "--tick-rate", "30", "--frame-rate", "30"]).unwrap();
    /// assert!(Args::try_from(cli).is_ok());
    ///
    /// let cli = Cli::try_parse_from(["dps", "--tick-rate", "60", "--frame-rate", "4"]).unwrap();
    /// assert!(Args::try_from(cli).is_err());
    /// ```
    fn try_from(cli: Cli) -> Result<Self, Self::Error> {
        if cli.frame_rate < cli.tick_rate {
            return Err(Cli::command().error(
                clap::error::ErrorKind::ArgumentConflict,
                format!(
                    "--frame-rate ({}) must be >= --tick-rate ({})",
                    cli.frame_rate, cli.tick_rate
                ),
            ));
        }

        Ok(Self {
            tick_rate: cli.tick_rate,
            frame_rate: cli.frame_rate,
            data_dir: cli.data_dir,
            config_dir: cli.config_dir,
        })
    }
}

/// Builds the version string shown by `--version`.
///
/// Includes the git tag or `<version>-<sha>`, build date, authors, and the
/// resolved config and data directory paths. Computed once and cached.
pub fn version() -> &'static str {
    VERSION.get_or_init(|| {
        let git_describe = env!("VERGEN_GIT_DESCRIBE");

        let version = if git_describe.is_empty() {
            format!("{}-{}", env!("CARGO_PKG_VERSION"), env!("VERGEN_GIT_SHA"))
        } else {
            git_describe.to_string()
        };

        let version_message = format!("{} ({})", version, env!("VERGEN_BUILD_DATE"));
        let author = clap::crate_authors!();

        format!(
            "\
{version_message}

Authors: {author}"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_relative_eq;
    use clap::Parser;
    use rstest::rstest;

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error(transparent)]
        Clap(#[from] clap::Error),
    }

    type TestResult<T> = Result<T, TestError>;

    mod try_parse_from {
        use super::*;

        #[rstest]
        fn defaults() -> TestResult<()> {
            let cli = Cli::try_parse_from(["dps"])?;

            assert_relative_eq!(cli.tick_rate, 4.0);
            assert_relative_eq!(cli.frame_rate, 60.0);

            assert!(!cli.data_dir.as_os_str().is_empty());
            assert!(!cli.config_dir.as_os_str().is_empty());

            Ok(())
        }

        #[rstest]
        fn long_flags() -> TestResult<()> {
            let cli = Cli::try_parse_from(["dps", "--tick-rate", "10.0", "--frame-rate", "30.0"])?;

            assert_relative_eq!(cli.tick_rate, 10.0);
            assert_relative_eq!(cli.frame_rate, 30.0);

            Ok(())
        }

        #[rstest]
        fn short_flags() -> TestResult<()> {
            let cli = Cli::try_parse_from(["dps", "-t", "8.0", "-f", "24.0"])?;

            assert_relative_eq!(cli.tick_rate, 8.0);
            assert_relative_eq!(cli.frame_rate, 24.0);

            Ok(())
        }

        #[rstest]
        fn data_and_config_dir_flags() -> TestResult<()> {
            let cli = Cli::try_parse_from([
                "dps",
                "--data-dir",
                "/tmp/mydata",
                "--config-dir",
                "/tmp/myconfig",
            ])?;

            assert_eq!(cli.data_dir, PathBuf::from("/tmp/mydata"));
            assert_eq!(cli.config_dir, PathBuf::from("/tmp/myconfig"));

            Ok(())
        }
    }

    mod parse_positive_hz {
        use super::*;

        #[rstest]
        fn zero_tick_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--tick-rate", "0"]).is_err());
        }

        #[rstest]
        fn negative_tick_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--tick-rate", "-1"]).is_err());
        }

        #[rstest]
        fn zero_frame_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--frame-rate", "0"]).is_err());
        }

        #[rstest]
        fn negative_frame_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--frame-rate", "-5"]).is_err());
        }
    }

    mod parse_dir {
        use super::*;

        #[rstest]
        fn empty_data_dir_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--data-dir", ""]).is_err());
        }

        #[rstest]
        fn empty_config_dir_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--config-dir", ""]).is_err());
        }
    }

    mod try_from {
        use super::*;

        #[rstest]
        fn frame_rate_equal_to_tick_rate_is_accepted() -> TestResult<()> {
            let cli = Cli::try_parse_from(["dps", "--tick-rate", "30", "--frame-rate", "30"])?;

            assert!(Args::try_from(cli).is_ok());

            Ok(())
        }

        #[rstest]
        fn frame_rate_below_tick_rate_is_rejected() -> TestResult<()> {
            let cli = Cli::try_parse_from(["dps", "--tick-rate", "60", "--frame-rate", "4"])?;

            assert!(Args::try_from(cli).is_err());

            Ok(())
        }
    }

    mod version {
        use super::*;

        #[rstest]
        fn contains_expected_sections() {
            let v = version();

            assert!(v.contains("Authors:"));
        }

        #[rstest]
        fn stable_across_calls() {
            assert_eq!(version(), version());
        }
    }
}
