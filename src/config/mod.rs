//! Platform-aware configuration and data directory resolution.
//!
//! ```
//! # fn main() -> Result<(), dps::config::ConfigError> {
//! use dps::config::Config;
//!
//! // An empty directory falls back to the embedded defaults.
//! let config = Config::from_dirs(std::env::temp_dir(), std::env::temp_dir())?;
//! let _theme = config.active_theme();
//! # Ok(())
//! # }
//! ```

pub mod error;
mod raw_config;
mod theme;

pub use self::error::Error as ConfigError;
pub use self::theme::ThemeMap;

use crate::{keymap::KeyBindings, theme::Theme};

use serde::Deserialize;
use tracing::error;

use std::path::{Path, PathBuf};

const CONFIG_FILES: &[&str] = [
    "config.json5",
    "config.json",
    "config.yaml",
    "config.yml",
    "config.toml",
]
.as_slice();

/// Paths written into [`Config`] by the config loader; override the
/// platform defaults via the `DPS_DATA` / `DPS_CONFIG` environment variables.
#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    /// Directory used for persistent data files such as logs.
    #[serde(default)]
    pub data_dir: PathBuf,
    /// Directory containing user configuration files.
    #[serde(default)]
    pub config_dir: PathBuf,
}

/// Top-level application configuration with all colours already resolved.
#[derive(Clone, Debug)]
pub struct Config {
    /// Resolved data and config directory paths.
    pub config: AppConfig,
    /// Key-sequence–to–action mappings loaded from the config file.
    pub keybindings: KeyBindings,
    /// All colour themes resolved at load time, keyed by the name used in the
    /// config file. The user can switch the active theme at runtime by updating
    /// `default_theme` to a key present in this map.
    ///
    /// User-defined themes must supply all 15 slot mappings; partial overrides
    /// of a built-in theme are not supported — define a new theme entry instead.
    pub themes: ThemeMap,
    /// Name of the currently active theme; must always be a key in `themes`.
    ///
    /// Switch themes at runtime by assigning a new name that exists in `themes`.
    /// Assigning an unknown name will cause [`active_theme`](Config::active_theme)
    /// to panic.
    pub default_theme: String,
}

/// Returns the built-in default [`Config`] using the embedded keybindings
/// and the Catppuccin Frappé theme.
///
/// # Examples
///
/// ```
/// use dps::config::Config;
///
/// let config = Config::default();
/// assert_eq!(config.default_theme, "catpuccineFrappe");
/// ```
impl Default for Config {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            keybindings: KeyBindings::default(),
            themes: ThemeMap::from([("catpuccineFrappe".to_string(), Theme::default())]),
            default_theme: "catpuccineFrappe".to_string(),
        }
    }
}

impl Config {
    /// Loads and merges configuration from the given directories.
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
    /// Theme priority: embedded defaults supply all four Catppuccin flavours.
    /// The user may add custom themes and palettes alongside them. All themes
    /// are resolved eagerly; `defaultTheme` must match a resolved theme name.
    ///
    /// # Errors
    ///
    /// Returns a parse error ([`error::Error::ParseJson`], [`error::Error::ParseYaml`],
    /// or [`error::Error::ParseToml`]) if a config file is present but cannot be
    /// parsed, [`error::Error::ThemeResolution`] if theme resolution fails (unknown
    /// palette colour or missing palette), or [`error::Error::UnknownTheme`] if
    /// `defaultTheme` does not match any resolved theme.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), dps::config::ConfigError> {
    /// use dps::config::Config;
    ///
    /// // An empty directory produces defaults; no error is returned.
    /// let config = Config::from_dirs(std::env::temp_dir(), std::env::temp_dir())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_dirs<P: AsRef<Path>>(config_dir: P, data_dir: P) -> Result<Self, ConfigError> {
        let base_config = raw_config::RawConfig::parse_base_config()?;

        let config_path = CONFIG_FILES
            .iter()
            .map(|name| config_dir.as_ref().join(name))
            .find(|p| p.exists());

        let mut config = if let Some(ref path) = config_path {
            raw_config::RawConfig::parse_config(path, &std::fs::read_to_string(path)?)?
        } else {
            error!("No configuration file found.");
            raw_config::RawConfig::default()
        };

        Self::try_from(raw_config::RawConfigContext {
            config: &mut config,
            base_config: &base_config,
            config_dir: config_dir.as_ref(),
            data_dir: data_dir.as_ref(),
        })
    }

    /// Returns the currently active theme.
    ///
    /// # Panics
    ///
    /// Panics if `default_theme` is not a key in `themes`. This invariant
    /// holds for any `Config` produced by [`Config::from_dirs`];
    /// it can be violated by assigning a name to
    /// `default_theme` that is not present in `themes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dps::config::Config;
    ///
    /// let config = Config::default();
    /// let _border_style = config.active_theme().border();
    /// ```
    #[must_use]
    pub fn active_theme(&self) -> &Theme {
        self.themes
            .get(&self.default_theme)
            .unwrap_or_else(|| unreachable!("invariant: default_theme is always a key in themes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        action::{Action, Movement},
        keymap::{Mode, keys::parse_key_sequence},
    };

    use rstest::rstest;

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error(transparent)]
        Config(#[from] ConfigError),
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error(transparent)]
        KeyMap(#[from] crate::keymap::KeyMapError),
        #[error("mode has no bindings")]
        MissingMode,
        #[error("key '{0}' not bound")]
        MissingKey(&'static str),
    }

    type TestResult<T = (), E = TestError> = std::result::Result<T, E>;

    mod keybindings {
        use super::*;

        #[rstest]
        fn default_keybindings_loaded_from_embedded_config() -> TestResult {
            let dir = tempfile::tempdir()?;
            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;

            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or(TestError::MissingKey("j"))?,
                &Action::Move(Movement::Down)
            );
            assert_eq!(
                home.get(&parse_key_sequence("gg")?)
                    .ok_or(TestError::MissingKey("gg"))?,
                &Action::Move(Movement::GotoTop)
            );

            Ok(())
        }

        #[rstest]
        fn user_config_adds_binding_and_defaults_merge_in() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ keybindings: { Normal: { x: "Move(ScrollUp)" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;

            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("x")?)
                    .ok_or(TestError::MissingKey("x"))?,
                &Action::Move(Movement::ScrollUp),
            );

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or(TestError::MissingKey("j"))?,
                &Action::Move(Movement::Down),
            );

            Ok(())
        }

        #[rstest]
        fn user_config_override_wins_over_default() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ keybindings: { Normal: { j: "Move(Up)" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;

            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or(TestError::MissingKey("j"))?,
                &Action::Move(Movement::Up),
            );

            Ok(())
        }

        #[rstest]
        fn from_dirs_loads_file_from_given_directory() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ keybindings: { Normal: { x: "Move(ScrollUp)" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;
            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("x")?)
                    .ok_or(TestError::MissingKey("x"))?,
                &Action::Move(Movement::ScrollUp),
            );

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or(TestError::MissingKey("j"))?,
                &Action::Move(Movement::Down),
            );

            Ok(())
        }

        #[rstest]
        fn leader_key_substitutes_in_bindings() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ leader: "<C-a>", keybindings: { Normal: { "<leader>j": "Quit" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;
            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("<C-a>j")?)
                    .ok_or(TestError::MissingKey("<C-a>j"))?,
                &Action::Quit,
            );

            Ok(())
        }

        #[rstest]
        fn malformed_config_returns_error() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                "{ this is not valid {{ json5 }",
            )?;

            assert!(Config::from_dirs(dir.path(), &std::env::temp_dir()).is_err());

            Ok(())
        }

        #[rstest]
        fn yaml_config_adds_binding_and_defaults_merge_in() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.yaml"),
                "keybindings:\n  Normal:\n    x: \"Move(ScrollUp)\"\n",
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;
            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("x")?)
                    .ok_or(TestError::MissingKey("x"))?,
                &Action::Move(Movement::ScrollUp),
            );
            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or(TestError::MissingKey("j"))?,
                &Action::Move(Movement::Down),
            );

            Ok(())
        }

        #[rstest]
        fn toml_config_adds_binding_and_defaults_merge_in() -> TestResult {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.toml"),
                "[keybindings.Normal]\nx = \"Move(ScrollUp)\"\n",
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;
            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or(TestError::MissingMode)?;

            assert_eq!(
                home.get(&parse_key_sequence("x")?)
                    .ok_or(TestError::MissingKey("x"))?,
                &Action::Move(Movement::ScrollUp),
            );
            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or(TestError::MissingKey("j"))?,
                &Action::Move(Movement::Down),
            );

            Ok(())
        }
    }
}
