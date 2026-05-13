//! Platform-aware configuration and data directory resolution.

use std::{env, path::PathBuf, sync::LazyLock};

use directories::ProjectDirs;
use serde::{Deserialize, de::Deserializer};
use tracing::error;

const CONFIG: &str = include_str!("../.config/config.json5");

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: Styles,
}

#[derive(Clone, Debug, Default)]
pub struct KeyBindings();

#[derive(Clone, Debug, Default)]
pub struct Styles();

pub static PROJECT_NAME: LazyLock<String> =
    LazyLock::new(|| env!("CARGO_CRATE_NAME").to_uppercase());
pub static DATA_FOLDER: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    env::var(format!("{}_DATA", *PROJECT_NAME))
        .ok()
        .map(PathBuf::from)
});
pub static CONFIG_FOLDER: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    env::var(format!("{}_CONFIG", *PROJECT_NAME))
        .ok()
        .map(PathBuf::from)
});

impl Config {
    pub fn new() -> color_eyre::Result<Self, config::ConfigError> {
        let _default_config: Config = json5::from_str(CONFIG).unwrap();
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        let mut builder = config::Config::builder()
            .set_default("data_dir", data_dir.to_str().unwrap())?
            .set_default("config_dir", config_dir.to_str().unwrap())?;

        let config_files = [
            ("config.json5", config::FileFormat::Json5),
            ("config.json", config::FileFormat::Json),
            ("config.yaml", config::FileFormat::Yaml),
            ("config.toml", config::FileFormat::Toml),
            ("config.ini", config::FileFormat::Ini),
        ];
        let mut found_config = false;
        for (file, format) in &config_files {
            let source = config::File::from(config_dir.join(file))
                .format(*format)
                .required(false);
            builder = builder.add_source(source);
            if config_dir.join(file).exists() {
                found_config = true
            }
        }
        if !found_config {
            error!("No configuration file found. Application may not behave as expected");
        }

        let cfg: Self = builder.build()?.try_deserialize()?;

        // this code needed for later
        // for (mode, default_bindings) in default_config.keybindings.0.iter() {
        // let user_bindings = cfg.keybindings.0.entry(*mode).or_default();
        // for (key, cmd) in default_bindings.iter() {
        //     user_bindings
        //         .entry(key.clone())
        //         .or_insert_with(|| cmd.clone());
        // }
        // }

        // this code needed for later
        // for (mode, default_styles) in default_config.styles.0.iter() {
        // let user_styles = cfg.styles.0.entry(*mode).or_default();
        // for (style_key, style) in default_styles.iter() {
        //     user_styles.entry(style_key.clone()).or_insert(*style);
        // }
        // }

        Ok(cfg)
    }
}

/// Returns the XDG/platform config directory for this application, or `None`
/// if the home directory cannot be determined.
fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
}

/// Returns the configuration directory.
///
/// Resolution order:
/// 1. `DPS_CONFIG` environment variable
/// 2. Platform config directory (`~/.config/dps` on Linux)
/// 3. `.config` in the current working directory
pub fn get_config_dir() -> PathBuf {
    let directory = if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    };
    directory
}

/// Returns the data directory used for logs and application state.
///
/// Resolution order:
/// 1. `DPS_DATA` environment variable
/// 2. Platform data directory (`~/.local/share/dps` on Linux)
/// 3. `.data` in the current working directory
pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(_deserializer: D) -> color_eyre::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(KeyBindings())
    }
}

impl<'de> Deserialize<'de> for Styles {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Styles())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // env::set_var / remove_var are process-global; serialize all env-touching
    // tests through this lock so parallel test threads don't race each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    mod get_config_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            let _guard = ENV_LOCK.lock().unwrap();
            unsafe { env::set_var("DPS_CONFIG", "/tmp/dps-test-config") };
            let dir = get_config_dir();
            unsafe { env::remove_var("DPS_CONFIG") };
            assert_eq!(dir, PathBuf::from("/tmp/dps-test-config"));
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            let _guard = ENV_LOCK.lock().unwrap();
            unsafe { env::remove_var("DPS_CONFIG") };
            assert!(!get_config_dir().as_os_str().is_empty());
        }
    }

    mod get_data_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            let _guard = ENV_LOCK.lock().unwrap();
            unsafe { env::set_var("DPS_DATA", "/tmp/dps-test-data") };
            let dir = get_data_dir();
            unsafe { env::remove_var("DPS_DATA") };
            assert_eq!(dir, PathBuf::from("/tmp/dps-test-data"));
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            let _guard = ENV_LOCK.lock().unwrap();
            unsafe { env::remove_var("DPS_DATA") };
            assert!(!get_data_dir().as_os_str().is_empty());
        }
    }
}
