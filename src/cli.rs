use std::sync::OnceLock;

use clap::Parser;

use crate::config::{get_config_dir, get_data_dir};

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,
}

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
