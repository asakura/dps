//! Platform-aware configuration and data directory resolution.

pub mod keys;
mod theme;

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

use crate::{action::Action, mode::Mode, theme::Theme};

use keys::parse_key_sequence;

const CONFIG: &str = include_str!("../../.config/config.json5");

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

/// Intermediate deserialization view of the config file.
///
/// Holds raw theme and palette maps long enough for [`theme::resolve_theme`]
/// to consume them; never exposed publicly.
#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default, flatten)]
    config: AppConfig,
    #[serde(default)]
    keybindings: KeyBindings,
    #[serde(default)]
    styles: Styles,
    #[serde(default, rename = "defaultTheme")]
    default_theme: String,
    #[serde(default)]
    themes: HashMap<String, theme::ThemeConfig>,
    #[serde(default)]
    palettes: HashMap<String, theme::PaletteConfig>,
}

/// Top-level application configuration with all colours already resolved.
#[derive(Clone, Debug)]
pub struct Config {
    /// Resolved data and config directory paths.
    pub config: AppConfig,
    /// Key-sequence–to–action mappings loaded from the config file.
    pub keybindings: KeyBindings,
    /// Reserved for future per-component style overrides.
    pub styles: Styles,
    /// All colour themes resolved at load time, keyed by the name used in the
    /// config file. The user can switch the active theme at runtime by updating
    /// `default_theme` to a key present in this map.
    ///
    /// User-defined themes must supply all 15 slot mappings; partial overrides
    /// of a built-in theme are not supported — define a new theme entry instead.
    pub themes: HashMap<String, Theme>,
    /// Name of the currently active theme; must always be a key in `themes`.
    ///
    /// Switch themes at runtime by assigning a new name that exists in `themes`.
    /// Assigning an unknown name will cause [`active_theme`](Config::active_theme)
    /// to panic.
    pub default_theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            keybindings: KeyBindings::default(),
            styles: Styles::default(),
            themes: HashMap::from([("catpuccineFrappe".to_string(), Theme::default())]),
            default_theme: "catpuccineFrappe".to_string(),
        }
    }
}

/// A two-level map from [`Mode`] → key sequence → [`Action`].
///
/// Deserialised from the `keybindings` table in the config file, where each
/// key sequence is a Vim-style string (see [`keys::parse_key_sequence`]).
#[derive(Clone, Debug, Default)]
pub struct KeyBindings(pub HashMap<Mode, HashMap<Vec<KeyEvent>, Action>>);

/// Placeholder for future style configuration.
#[derive(Clone, Copy, Debug, Default)]
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
    ///
    /// # Errors
    ///
    /// Returns `Err` if the config source cannot be read or parsed; see
    /// [`Config::from_dirs`].
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
    /// Theme priority: embedded defaults supply all four Catppuccin flavours.
    /// The user may add custom themes and palettes alongside them. All themes
    /// are resolved eagerly; `defaultTheme` must match a resolved theme name.
    ///
    /// # Errors
    ///
    /// Returns a [`config::ConfigError`] if a config file is present but
    /// cannot be parsed or deserialised, if theme resolution fails (unknown
    /// palette colour or missing palette), or if `defaultTheme` does not
    /// match any resolved theme.
    pub fn from_dirs(
        config_dir: Option<&Path>,
        data_dir: Option<&Path>,
    ) -> color_eyre::Result<Self, config::ConfigError> {
        let default_raw: RawConfig =
            json5::from_str(CONFIG).map_err(|e| config::ConfigError::Message(e.to_string()))?;

        let effective_data_dir = data_dir.map_or_else(get_data_dir, Path::to_path_buf);
        let effective_config_dir = config_dir.map_or_else(get_config_dir, Path::to_path_buf);

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

        let mut raw: RawConfig = builder.build()?.try_deserialize()?;

        for (mode, default_bindings) in &default_raw.keybindings.0 {
            let user_bindings = raw.keybindings.0.entry(*mode).or_default();
            for (key, cmd) in default_bindings {
                user_bindings.entry(key.clone()).or_insert(*cmd);
            }
        }

        // TODO: merge default styles once Styles carries real data.

        if raw.default_theme.is_empty() {
            raw.default_theme.clone_from(&default_raw.default_theme);
        }
        for (name, t) in &default_raw.themes {
            raw.themes.entry(name.clone()).or_insert_with(|| t.clone());
        }
        for (name, p) in &default_raw.palettes {
            raw.palettes
                .entry(name.clone())
                .or_insert_with(|| p.clone());
        }

        let themes = theme::resolve_theme(&raw.themes, &raw.palettes)
            .map_err(|e| config::ConfigError::Message(e.to_string()))?;

        if !themes.contains_key(&raw.default_theme) {
            return Err(config::ConfigError::Message(format!(
                "defaultTheme '{}' does not match any resolved theme",
                raw.default_theme,
            )));
        }

        Ok(Self {
            config: raw.config,
            keybindings: raw.keybindings,
            styles: raw.styles,
            themes,
            default_theme: raw.default_theme,
        })
    }

    /// Returns the currently active theme.
    ///
    /// # Panics
    ///
    /// Panics if `default_theme` is not a key in `themes`. This invariant
    /// holds for any `Config` produced by [`Config::new`] or
    /// [`Config::from_dirs`]; it can be violated by assigning a name to
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
#[must_use]
pub fn get_config_dir() -> PathBuf {
    env::var(format!("{}_CONFIG", *PROJECT_NAME)).map_or_else(
        |_| {
            project_directory().map_or_else(
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
#[must_use]
pub fn get_data_dir() -> PathBuf {
    env::var(format!("{}_DATA", *PROJECT_NAME)).map_or_else(
        |_| {
            project_directory().map_or_else(
                || PathBuf::from(".").join(".data"),
                |d| d.data_local_dir().to_path_buf(),
            )
        },
        PathBuf::from,
    )
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

        Ok(Self(keybindings))
    }
}

impl<'de> Deserialize<'de> for Styles {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_config_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            temp_env::with_var("DPS_CONFIG", Some("/tmp/dps-test-config"), || {
                assert_eq!(get_config_dir(), PathBuf::from("/tmp/dps-test-config"));
            });
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            temp_env::with_var_unset("DPS_CONFIG", || {
                assert!(!get_config_dir().as_os_str().is_empty());
            });
        }
    }

    mod get_data_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            temp_env::with_var("DPS_DATA", Some("/tmp/dps-test-data"), || {
                assert_eq!(get_data_dir(), PathBuf::from("/tmp/dps-test-data"));
            });
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            temp_env::with_var_unset("DPS_DATA", || {
                assert!(!get_data_dir().as_os_str().is_empty());
            });
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
    fn user_config_adds_binding_and_defaults_merge_in() {
        use crate::action::Movement;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{ keybindings: { Home: { x: "ScrollUp" } } }"#,
        )
        .unwrap();
        temp_env::with_var("DPS_CONFIG", Some(dir.path()), || {
            let c = Config::new().unwrap();
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
        });
    }

    #[test]
    fn user_config_override_wins_over_default() {
        use crate::action::Movement;
        let dir = tempfile::tempdir().unwrap();
        // Remap 'j' from Down (default) to Up.
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{ keybindings: { Home: { j: "Up" } } }"#,
        )
        .unwrap();
        temp_env::with_var("DPS_CONFIG", Some(dir.path()), || {
            let c = Config::new().unwrap();
            let home = c.keybindings.0.get(&Mode::Home).unwrap();
            // User's remap wins — default must not overwrite it.
            assert_eq!(
                home.get(&parse_key_sequence("j").unwrap()).unwrap(),
                &Action::Move(Movement::Up),
            );
        });
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

    #[test]
    fn theme_resolved_from_embedded_config() -> color_eyre::Result<()> {
        use ratatui::style::{Color, Modifier, Style};
        let c = Config::new()?;
        // Default is catpuccineFrappe; peach = #ef9f76 = Rgb(239, 159, 118).
        let theme = c.themes.get(&c.default_theme).unwrap();
        assert_eq!(
            theme.key_label(),
            Style::from((Color::Rgb(239, 159, 118), Modifier::BOLD)),
        );
        Ok(())
    }

    #[test]
    fn all_four_catppuccin_themes_resolved() -> color_eyre::Result<()> {
        let c = Config::new()?;
        for name in [
            "catpuccineLatte",
            "catpuccineFrappe",
            "catpuccineMacchiato",
            "catpuccineMocha",
        ] {
            assert!(c.themes.contains_key(name), "missing theme '{name}'");
        }
        Ok(())
    }

    #[test]
    fn unknown_default_theme_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{ defaultTheme: "doesNotExist" }"#,
        )
        .unwrap();
        let err = Config::from_dirs(Some(dir.path()), None).unwrap_err();
        assert!(
            err.to_string().contains("doesNotExist"),
            "error should name the unknown theme; got: {err}",
        );
    }

    #[test]
    fn theme_without_palette_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        // A fully-specified theme with no matching palette entry.
        std::fs::write(
            dir.path().join("config.json5"),
            r#"{
              themes: {
                orphanTheme: {
                  popupSurface: { fg: "text",     bg: "mantle"   },
                  keyLabel:     { fg: "peach"                    },
                  border:       { fg: "surface0"                 },
                  title:        { fg: "lavender"                 },
                  header:       { fg: "blue"                     },
                  selection:    { fg: "base",     bg: "mauve"    },
                  columnFocus:  { fg: "lavender"                 },
                  navBar:       { fg: "subtext0", bg: "surface0" },
                  statusActive: { fg: "text",     bg: "surface0" },
                  statusEmpty:  { fg: "overlay0", bg: "surface0" },
                  safe:         { fg: "green"                    },
                  caution:      { fg: "yellow"                   },
                  danger:       { fg: "red"                      },
                  bodyText:     { fg: "text"                     },
                  hint:         { fg: "subtext0"                 },
                }
              }
            }"#,
        )
        .unwrap();
        assert!(Config::from_dirs(Some(dir.path()), None).is_err());
    }
}
