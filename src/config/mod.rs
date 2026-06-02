//! Platform-aware configuration and data directory resolution.

pub mod error;
mod theme;

pub use self::error::Error as ConfigError;

use crate::{
    keymap::{KeyBindings, KeyBindingsBuilder, keys::parse_key_sequence},
    theme::Theme,
};

use color_eyre::Result;
use serde::{Deserialize, de::Deserializer};
use tracing::error;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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

fn default_leader() -> String {
    "<Space>".to_string()
}

/// Intermediate deserialization view of the config file.
///
/// Holds raw theme and palette maps long enough for [`theme::resolve_theme`]
/// to consume them; never exposed publicly.
#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default, flatten)]
    config: AppConfig,
    /// Leader key used to resolve `<leader>` tokens in keybinding sequences.
    /// Accepts any key sequence string understood by `parse_key_sequence`,
    /// e.g. `"<Space>"`, `","`, `"<C-a>"`. Defaults to `"<Space>"`.
    #[serde(default = "default_leader")]
    leader: String,
    #[serde(default)]
    keybindings: KeyBindingsBuilder,
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

/// Placeholder for future style configuration.
#[derive(Clone, Copy, Debug, Default)]
pub struct Styles();

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
    /// Returns [`error::Error::Load`] if a config file is present but
    /// cannot be parsed or deserialised, if theme resolution fails (unknown
    /// palette colour or missing palette), or if `defaultTheme` does not
    /// match any resolved theme.
    pub fn from_dirs<P: AsRef<Path>>(config_dir: P, data_dir: P) -> Result<Self, ConfigError> {
        let default_raw: RawConfig = json5::from_str(CONFIG)?;

        let mut builder = config::Config::builder()
            .set_default("data_dir", data_dir.as_ref().to_string_lossy().into_owned())?
            .set_default(
                "config_dir",
                config_dir.as_ref().to_string_lossy().into_owned(),
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
            let source = config::File::from(config_dir.as_ref().join(file))
                .format(*format)
                .required(false);

            builder = builder.add_source(source);

            if config_dir.as_ref().join(file).exists() {
                found_config = true;
            }
        }

        if !found_config {
            error!("No configuration file found. Application may not behave as expected");
        }

        let mut raw: RawConfig = builder.build()?.try_deserialize()?;

        parse_key_sequence(&raw.leader)?;

        raw.keybindings.merge_defaults(&default_raw.keybindings);

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

        let themes = theme::resolve_theme(&raw.themes, &raw.palettes)?;

        if !themes.contains_key(&raw.default_theme) {
            return Err(error::Error::UnknownTheme(raw.default_theme));
        }

        Ok(Self {
            config: raw.config,
            keybindings: raw.keybindings.build_with_leader(&raw.leader),
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

    use crate::action::{Action, Movement};
    use crate::keymap::{Mode, keys::parse_key_sequence};

    use color_eyre::eyre::eyre;

    mod keybindings {
        use super::*;
        use rstest::rstest;

        #[test]
        fn default_keybindings_loaded_from_embedded_config() -> Result<()> {
            let dir = tempfile::tempdir()?;
            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;

            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or_else(|| eyre!("no Normal bindings in config"))?;

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or_else(|| eyre!("no binding for 'j'"))?,
                &Action::Move(Movement::Down)
            );
            assert_eq!(
                home.get(&parse_key_sequence("gg")?)
                    .ok_or_else(|| eyre!("no binding for 'gg'"))?,
                &Action::Move(Movement::GotoTop)
            );

            Ok(())
        }

        #[test]
        fn user_config_adds_binding_and_defaults_merge_in() -> Result<()> {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ keybindings: { Normal: { x: "Move(ScrollUp)" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;

            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or_else(|| eyre!("no Normal bindings in config"))?;

            assert_eq!(
                home.get(&parse_key_sequence("x")?)
                    .ok_or_else(|| eyre!("no binding for 'x'"))?,
                &Action::Move(Movement::ScrollUp),
            );

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or_else(|| eyre!("no binding for 'j'"))?,
                &Action::Move(Movement::Down),
            );

            Ok(())
        }

        #[test]
        fn user_config_override_wins_over_default() -> Result<()> {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ keybindings: { Normal: { j: "Move(Up)" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;

            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or_else(|| eyre!("no Normal bindings in config"))?;

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or_else(|| eyre!("no binding for 'j'"))?,
                &Action::Move(Movement::Up),
            );

            Ok(())
        }

        #[test]
        fn from_dirs_loads_file_from_given_directory() -> Result<()> {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ keybindings: { Normal: { x: "Move(ScrollUp)" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;
            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or_else(|| eyre!("no Normal bindings in config"))?;

            assert_eq!(
                home.get(&parse_key_sequence("x")?)
                    .ok_or_else(|| eyre!("no binding for 'x'"))?,
                &Action::Move(Movement::ScrollUp),
            );

            assert_eq!(
                home.get(&parse_key_sequence("j")?)
                    .ok_or_else(|| eyre!("no binding for 'j'"))?,
                &Action::Move(Movement::Down),
            );

            Ok(())
        }

        #[rstest]
        fn leader_key_substitutes_in_bindings() -> Result<()> {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                r#"{ leader: "<C-a>", keybindings: { Normal: { "<leader>j": "Quit" } } }"#,
            )?;

            let c = Config::from_dirs(dir.path(), &std::env::temp_dir())?;
            let home = c
                .keybindings
                .get(&Mode::Normal)
                .ok_or_else(|| eyre!("no Normal bindings in config"))?;

            assert_eq!(
                home.get(&parse_key_sequence("<C-a>j")?)
                    .ok_or_else(|| eyre!("no binding for <C-a>j"))?,
                &Action::Quit,
            );

            Ok(())
        }

        #[test]
        fn malformed_config_returns_error() -> Result<()> {
            let dir = tempfile::tempdir()?;

            std::fs::write(
                dir.path().join("config.json5"),
                "{ this is not valid {{ json5 }",
            )?;

            assert!(Config::from_dirs(dir.path(), &std::env::temp_dir()).is_err());

            Ok(())
        }
    }
}
