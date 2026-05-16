//! Command-line interface: argument parsing and version string construction.

use std::{path::PathBuf, sync::OnceLock};

use clap::{CommandFactory, Parser};

use crate::config::{get_config_dir, get_data_dir};

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

/// Command-line arguments for `dps`.
#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Input-polling rate in Hz — how often the event loop checks for key presses.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0, value_parser = parse_positive_hz)]
    pub tick_rate: f64,

    /// Render rate in Hz — maximum number of frames drawn per second.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0, value_parser = parse_positive_hz)]
    pub frame_rate: f64,

    /// Override the data directory (logs, state). Defaults to the platform data dir
    /// or the `DPS_DATA` environment variable.
    #[arg(long, value_name = "PATH")]
    pub data_dir: Option<PathBuf>,

    /// Override the config directory. Defaults to the platform config dir
    /// or the `DPS_CONFIG` environment variable.
    #[arg(long, value_name = "PATH")]
    pub config_dir: Option<PathBuf>,
}

impl Cli {
    /// Returns a clap error if `frame_rate` is less than `tick_rate`.
    ///
    /// # Errors
    ///
    /// Returns a [`clap::Error`] of kind `ArgumentConflict` if `frame_rate < tick_rate`.
    pub fn validate(&self) -> Result<(), clap::Error> {
        if self.frame_rate < self.tick_rate {
            return Err(Self::command().error(
                clap::error::ErrorKind::ArgumentConflict,
                format!(
                    "--frame-rate ({}) must be >= --tick-rate ({})",
                    self.frame_rate, self.tick_rate
                ),
            ));
        }
        Ok(())
    }
}

/// Builds the version string shown by `--version`.
///
/// Includes the git tag or `<version>-<sha>`, build date, authors, and the
/// resolved config and data directory paths. Computed once and cached.
pub fn version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let git_describe = env!("VERGEN_GIT_DESCRIBE");
        let version = if git_describe.is_empty() {
            format!("{}-{}", env!("CARGO_PKG_VERSION"), env!("VERGEN_GIT_SHA"))
        } else {
            git_describe.to_string()
        };
        let version_message = format!("{} ({})", version, env!("VERGEN_BUILD_DATE"));
        let author = clap::crate_authors!();
        let config_dir_path = get_config_dir().display().to_string();
        let data_dir_path = get_data_dir().display().to_string();
        format!(
            "\
{version_message}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    mod cli_args {
        use super::*;

        #[test]
        fn defaults() {
            let cli = Cli::try_parse_from(["dps"]).unwrap();
            assert_eq!(cli.tick_rate, 4.0);
            assert_eq!(cli.frame_rate, 60.0);
            assert!(cli.data_dir.is_none());
            assert!(cli.config_dir.is_none());
        }

        #[test]
        fn data_and_config_dir_flags() {
            let cli = Cli::try_parse_from([
                "dps",
                "--data-dir",
                "/tmp/mydata",
                "--config-dir",
                "/tmp/myconfig",
            ])
            .unwrap();
            assert_eq!(cli.data_dir.unwrap(), PathBuf::from("/tmp/mydata"));
            assert_eq!(cli.config_dir.unwrap(), PathBuf::from("/tmp/myconfig"));
        }

        #[test]
        fn long_flags() {
            let cli = Cli::try_parse_from(["dps", "--tick-rate", "10.0", "--frame-rate", "30.0"])
                .unwrap();
            assert_eq!(cli.tick_rate, 10.0);
            assert_eq!(cli.frame_rate, 30.0);
        }

        #[test]
        fn short_flags() {
            let cli = Cli::try_parse_from(["dps", "-t", "8.0", "-f", "24.0"]).unwrap();
            assert_eq!(cli.tick_rate, 8.0);
            assert_eq!(cli.frame_rate, 24.0);
        }

        #[test]
        fn frame_rate_below_tick_rate_is_rejected() {
            let cli =
                Cli::try_parse_from(["dps", "--tick-rate", "60", "--frame-rate", "4"]).unwrap();
            assert!(cli.validate().is_err());
        }

        #[test]
        fn frame_rate_equal_to_tick_rate_is_accepted() {
            let cli =
                Cli::try_parse_from(["dps", "--tick-rate", "30", "--frame-rate", "30"]).unwrap();
            assert!(cli.validate().is_ok());
        }

        #[test]
        fn zero_tick_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--tick-rate", "0"]).is_err());
        }

        #[test]
        fn negative_tick_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--tick-rate", "-1"]).is_err());
        }

        #[test]
        fn zero_frame_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--frame-rate", "0"]).is_err());
        }

        #[test]
        fn negative_frame_rate_is_rejected() {
            assert!(Cli::try_parse_from(["dps", "--frame-rate", "-5"]).is_err());
        }
    }

    mod version_fn {
        use super::*;

        #[test]
        fn contains_expected_sections() {
            let v = version();
            assert!(v.contains("Authors:"));
            assert!(v.contains("Config directory:"));
            assert!(v.contains("Data directory:"));
        }

        #[test]
        fn stable_across_calls() {
            assert_eq!(version(), version());
        }
    }
}
