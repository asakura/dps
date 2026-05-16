//! Platform-aware configuration and data directory resolution.

pub mod keys;

use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crossterm::event::KeyEvent;
use directories::ProjectDirs;
use serde::{Deserialize, de::Deserializer};
use tracing::error;

use crate::{action::Action, mode::Mode};

use keys::parse_key_sequence;

const CONFIG: &str = include_str!("../../.config/config.json5");

/// Paths written into [`Config`] by the config loader; override the
/// platform defaults via the `DPS_DATA` / `DPS_CONFIG` environment variables.
#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
}

/// Top-level application configuration, deserialised from the user's config
/// file and merged with the embedded defaults.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: Styles,
}

/// A two-level map from [`Mode`] → key sequence → [`Action`].
///
/// Deserialised from the `keybindings` table in the config file, where each
/// key sequence is a Vim-style string (see [`keys::parse_key_sequence`]).
#[derive(Clone, Debug, Default)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

/// Placeholder for future style configuration.
#[derive(Clone, Debug, Default)]
pub struct Styles();

/// Upper-cased crate name, used as the prefix for environment variables
/// (`DPS_DATA`, `DPS_CONFIG`, `DPS_LOG_LEVEL`).
pub static PROJECT_NAME: LazyLock<String> =
    LazyLock::new(|| env!("CARGO_CRATE_NAME").to_uppercase());

/// Value of the `DPS_DATA` environment variable at process start, if set.
///
/// `None` means no override — the platform default from [`get_data_dir`] is in use.
/// Consumed at startup for diagnostic logging.
pub static DATA_FOLDER: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    env::var(format!("{}_DATA", *PROJECT_NAME))
        .ok()
        .map(PathBuf::from)
});

/// Value of the `DPS_CONFIG` environment variable at process start, if set.
///
/// `None` means no override — the platform default from [`get_config_dir`] is in use.
/// Consumed at startup for diagnostic logging.
pub static CONFIG_FOLDER: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    env::var(format!("{}_CONFIG", *PROJECT_NAME))
        .ok()
        .map(PathBuf::from)
});

impl Config {
    /// Loads config from the env-var / platform-default directories.
    pub fn new() -> color_eyre::Result<Self, config::ConfigError> {
        Self::from_dirs(None, None)
    }

    /// Loads and merges configuration, optionally overriding the config and
    /// data directories.  `None` falls back to the env-var / platform default
    /// (same as [`Config::new`]).
    ///
    /// Directory priority (highest → lowest):
    /// 1. `config_dir` / `data_dir` parameters (CLI flags)
    /// 2. User config file values for `config_dir` / `data_dir`
    /// 3. `DPS_CONFIG` / `DPS_DATA` environment variables
    /// 4. Platform default directories
    ///
    /// Keybinding priority: user file overrides embedded defaults; missing
    /// keys are filled from the embedded defaults so partial user configs work.
    ///
    /// # Errors
    ///
    /// Returns a [`config::ConfigError`] if a config file is present but
    /// cannot be parsed or deserialised.
    pub fn from_dirs(
        config_dir: Option<&Path>,
        data_dir: Option<&Path>,
    ) -> color_eyre::Result<Self, config::ConfigError> {
        let default_config: Config =
            json5::from_str(CONFIG).expect("embedded config.json5 is malformed");

        let effective_data_dir = data_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(get_data_dir);
        let effective_config_dir = config_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(get_config_dir);

        let mut builder = config::Config::builder()
            .set_default(
                "data_dir",
                effective_data_dir.to_string_lossy().into_owned(),
            )?
            .set_default(
                "config_dir",
                effective_config_dir.to_string_lossy().into_owned(),
            )?;

        let config_files = [
            ("config.json5", config::FileFormat::Json5),
            ("config.json", config::FileFormat::Json),
            ("config.yaml", config::FileFormat::Yaml),
            ("config.toml", config::FileFormat::Toml),
            ("config.ini", config::FileFormat::Ini),
        ];
        let mut found_config = false;
        for (file, format) in &config_files {
            let source = config::File::from(effective_config_dir.join(file))
                .format(*format)
                .required(false);
            builder = builder.add_source(source);
            if effective_config_dir.join(file).exists() {
                found_config = true;
            }
        }
        if !found_config {
            error!("No configuration file found. Application may not behave as expected");
        }

        // Explicit directory parameters win over anything the config file may set.
        if let Some(p) = data_dir {
            builder = builder.set_override("data_dir", p.to_string_lossy().into_owned())?;
        }
        if let Some(p) = config_dir {
            builder = builder.set_override("config_dir", p.to_string_lossy().into_owned())?;
        }

        let mut cfg: Self = builder.build()?.try_deserialize()?;

        for (mode, default_bindings) in default_config.keybindings.0.iter() {
            let user_bindings = cfg.keybindings.0.entry(*mode).or_default();
            for (key, cmd) in default_bindings.iter() {
                user_bindings.entry(key.clone()).or_insert_with(|| cmd.clone());
            }
        }

        // TODO: merge default styles once Styles carries real data.

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
///
/// The env variable is re-read on every call so that tests can override it
/// in isolation without racing against each other.
///
/// # Examples
///
/// ```
/// use dps::config::get_config_dir;
///
/// assert!(!get_config_dir().as_os_str().is_empty());
/// ```
pub fn get_config_dir() -> PathBuf {
    if let Ok(s) = env::var(format!("{}_CONFIG", *PROJECT_NAME)) {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    }
}

/// Returns the data directory used for logs and application state.
///
/// Resolution order:
/// 1. `DPS_DATA` environment variable
/// 2. Platform data directory (`~/.local/share/dps` on Linux)
/// 3. `.data` in the current working directory
///
/// The env variable is re-read on every call so that tests can override it
/// in isolation without racing against each other.
///
/// # Examples
///
/// ```
/// use dps::config::get_data_dir;
///
/// assert!(!get_data_dir().as_os_str().is_empty());
/// ```
pub fn get_data_dir() -> PathBuf {
    if let Ok(s) = env::var(format!("{}_DATA", *PROJECT_NAME)) {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    }
}

impl<'de> Deserialize<'de> for KeyBindings {
    fn deserialize<D>(deserializer: D) -> color_eyre::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<Mode, HashMap<String, Action>>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(mode, inner_map)| {
                let converted_inner_map = inner_map
                    .into_iter()
                    .map(|(key_str, cmd)| {
                        let seq = parse_key_sequence(&key_str).map_err(serde::de::Error::custom)?;
                        Ok((seq, cmd))
                    })
                    .collect::<Result<HashMap<Vec<KeyEvent>, Action>, D::Error>>()?;

                Ok((mode, converted_inner_map))
            })
            .collect::<Result<HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>, D::Error>>()?;

        Ok(KeyBindings(keybindings))
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
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            unsafe { env::set_var("DPS_CONFIG", "/tmp/dps-test-config") };
            let dir = get_config_dir();
            unsafe { env::remove_var("DPS_CONFIG") };
            assert_eq!(dir, PathBuf::from("/tmp/dps-test-config"));
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            unsafe { env::remove_var("DPS_CONFIG") };
            assert!(!get_config_dir().as_os_str().is_empty());
        }
    }

    mod get_data_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            unsafe { env::set_var("DPS_DATA", "/tmp/dps-test-data") };
            let dir = get_data_dir();
            unsafe { env::remove_var("DPS_DATA") };
            assert_eq!(dir, PathBuf::from("/tmp/dps-test-data"));
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            unsafe { env::remove_var("DPS_DATA") };
            assert!(!get_data_dir().as_os_str().is_empty());
        }
    }

    #[test]
    fn default_keybindings_loaded_from_embedded_config() -> color_eyre::Result<()> {
        use crate::action::Movement;
        let c = Config::new()?;
        let home = c.keybindings.0.get(&Mode::Home).unwrap();
        // Spot-check a few bindings that must come from the embedded config.json5.
        assert_eq!(
            home.get(&parse_key_sequence("j").unwrap()).unwrap(),
            &Action::Move(Movement::Down)
        );
        assert_eq!(
            home.get(&parse_key_sequence("gg").unwrap()).unwrap(),
            &Action::Move(Movement::GotoTop)
        );
        Ok(())
    }

    #[test]
    fn user_config_adds_binding_and_defaults_merge_in() -> color_eyre::Result<()> {
        use crate::action::Movement;
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{ keybindings: { Home: { x: "ScrollUp" } } }"#,
        )?;
        unsafe { env::set_var("DPS_CONFIG", dir.path()) };
        let result = Config::new();
        unsafe { env::remove_var("DPS_CONFIG") };

        let c = result?;
        let home = c.keybindings.0.get(&Mode::Home).unwrap();
        assert_eq!(
            home.get(&parse_key_sequence("x").unwrap()).unwrap(),
            &Action::Move(Movement::ScrollUp),
        );
        // Default binding merged in alongside the user binding.
        assert_eq!(
            home.get(&parse_key_sequence("j").unwrap()).unwrap(),
            &Action::Move(Movement::Down),
        );
        Ok(())
    }

    #[test]
    fn user_config_override_wins_over_default() -> color_eyre::Result<()> {
        use crate::action::Movement;
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let dir = tempfile::tempdir().unwrap();
        // Remap 'j' from Down (default) to Up.
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{ keybindings: { Home: { j: "Up" } } }"#,
        )?;
        unsafe { env::set_var("DPS_CONFIG", dir.path()) };
        let result = Config::new();
        unsafe { env::remove_var("DPS_CONFIG") };

        let c = result?;
        let home = c.keybindings.0.get(&Mode::Home).unwrap();
        // User's remap wins — default must not overwrite it.
        assert_eq!(
            home.get(&parse_key_sequence("j").unwrap()).unwrap(),
            &Action::Move(Movement::Up),
        );
        Ok(())
    }

    #[test]
    fn from_dirs_loads_file_from_given_directory() -> color_eyre::Result<()> {
        use crate::action::Movement;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{ keybindings: { Home: { x: "ScrollUp" } } }"#,
        )?;
        let c = Config::from_dirs(Some(dir.path()), None)?;
        let home = c.keybindings.0.get(&Mode::Home).unwrap();
        // User-added binding present.
        assert_eq!(
            home.get(&parse_key_sequence("x").unwrap()).unwrap(),
            &Action::Move(Movement::ScrollUp),
        );
        // Embedded default still merged in.
        assert_eq!(
            home.get(&parse_key_sequence("j").unwrap()).unwrap(),
            &Action::Move(Movement::Down),
        );
        Ok(())
    }

    #[test]
    fn malformed_config_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json5"),
            "{ this is not valid {{ json5 }",
        )
        .unwrap();
        assert!(Config::from_dirs(Some(dir.path()), None).is_err());
    }
}
