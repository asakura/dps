//! Command-line interface: argument parsing and version string construction.

use std::sync::OnceLock;

use clap::Parser;

use crate::config::{get_config_dir, get_data_dir};

/// Command-line arguments for `dps`.
#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Input-polling rate in Hz — how often the event loop checks for key presses.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Render rate in Hz — maximum number of frames drawn per second.
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,
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
        }

        #[test]
        fn long_flags() {
            let cli = Cli::try_parse_from(["dps", "--tick-rate", "10.0", "--frame-rate", "30.0"]).unwrap();
            assert_eq!(cli.tick_rate, 10.0);
            assert_eq!(cli.frame_rate, 30.0);
        }

        #[test]
        fn short_flags() {
            let cli = Cli::try_parse_from(["dps", "-t", "8.0", "-f", "24.0"]).unwrap();
            assert_eq!(cli.tick_rate, 8.0);
            assert_eq!(cli.frame_rate, 24.0);
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
