use super::theme;
use super::{AppConfig, Config, ConfigError};

use crate::keymap::KeyBindingsBuilder;

use serde::Deserialize;

use std::path::Path;

const BASE_CONFIG_CONTENT: &str = include_str!("../../.config/config.json5");

/// Intermediate deserialization view of the config file.
///
/// Holds raw theme and palette maps long enough for [`theme::resolve_theme`]
/// to consume them; never exposed publicly.
#[derive(Deserialize, Default)]
pub(super) struct RawConfig {
    /// Leader key used to resolve `<leader>` tokens in keybinding sequences.
    /// Accepts any key sequence string understood by `parse_key_sequence`,
    /// e.g. `"<Space>"`, `","`, `"<C-a>"`. Defaults to `"<Space>"`.
    #[serde(default = "RawConfig::default_leader")]
    leader: String,
    #[serde(default)]
    keybindings: KeyBindingsBuilder,
    #[serde(default, rename = "defaultTheme")]
    default_theme: String,
    #[serde(default)]
    themes: theme::ThemeConfigMap,
    #[serde(default)]
    palettes: theme::PaletteConfigMap,
}

impl RawConfig {
    fn default_leader() -> String {
        "<Space>".to_string()
    }

    pub fn parse_base_config() -> Result<Self, ConfigError> {
        json5::from_str(BASE_CONFIG_CONTENT).map_err(ConfigError::EmbeddedConfig)
    }

    pub fn parse_config(path: &Path, content: &str) -> Result<Self, ConfigError> {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("json5") {
            "yaml" | "yml" => serde_saphyr::from_str(content).map_err(ConfigError::ParseYaml),
            "toml" => toml::from_str(content).map_err(ConfigError::ParseToml),
            _ => json5::from_str(content).map_err(ConfigError::ParseJson),
        }
    }
}

pub(super) struct RawConfigContext<'a> {
    pub(super) config: &'a mut RawConfig,
    pub(super) base_config: &'a RawConfig,
    pub(super) config_dir: &'a Path,
    pub(super) data_dir: &'a Path,
}

impl<'a> TryFrom<RawConfigContext<'a>> for Config {
    type Error = ConfigError;

    fn try_from(ctx: RawConfigContext<'a>) -> Result<Self, Self::Error> {
        let RawConfigContext {
            config,
            base_config,
            config_dir,
            data_dir,
        } = ctx;

        config.keybindings.merge_defaults(&base_config.keybindings);

        if config.default_theme.is_empty() {
            config.default_theme.clone_from(&base_config.default_theme);
        }

        for (name, t) in &base_config.themes {
            config
                .themes
                .entry(name.clone())
                .or_insert_with(|| t.clone());
        }

        for (name, p) in &base_config.palettes {
            config
                .palettes
                .entry(name.clone())
                .or_insert_with(|| p.clone());
        }

        let themes = theme::resolve_theme(&config.themes, &config.palettes)?;

        if !themes.contains_key(&config.default_theme) {
            return Err(ConfigError::UnknownTheme(std::mem::take(
                &mut config.default_theme,
            )));
        }

        Ok(Self {
            config: AppConfig {
                config_dir: config_dir.to_path_buf(),
                data_dir: data_dir.to_path_buf(),
            },
            keybindings: config.keybindings.build_with_leader(&config.leader),
            themes,
            default_theme: std::mem::take(&mut config.default_theme),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    use std::path::Path;

    mod parse_base_config {
        use super::*;

        #[rstest]
        fn embedded_config_is_valid() -> Result<(), ConfigError> {
            RawConfig::parse_base_config()?;
            Ok(())
        }
    }

    mod parse_config {
        use super::*;

        #[rstest]
        fn json5_parses_empty() -> Result<(), ConfigError> {
            RawConfig::parse_config(Path::new("config.json5"), "{}")?;
            Ok(())
        }

        #[rstest]
        fn yaml_parses_empty() -> Result<(), ConfigError> {
            RawConfig::parse_config(Path::new("config.yaml"), "{}")?;
            Ok(())
        }

        #[rstest]
        fn yml_parses_empty() -> Result<(), ConfigError> {
            RawConfig::parse_config(Path::new("config.yml"), "{}")?;
            Ok(())
        }

        #[rstest]
        fn toml_parses_empty() -> Result<(), ConfigError> {
            RawConfig::parse_config(Path::new("config.toml"), "")?;
            Ok(())
        }

        #[rstest]
        fn unknown_extension_falls_back_to_json5() -> Result<(), ConfigError> {
            RawConfig::parse_config(Path::new("config.cfg"), "{}")?;
            Ok(())
        }

        #[rstest]
        fn invalid_json5_is_err() {
            assert!(RawConfig::parse_config(Path::new("config.json5"), "{ not valid {{").is_err());
        }

        #[rstest]
        fn invalid_yaml_is_err() {
            assert!(RawConfig::parse_config(Path::new("config.yaml"), "key: [invalid}").is_err());
        }

        #[rstest]
        fn invalid_toml_is_err() {
            assert!(RawConfig::parse_config(Path::new("config.toml"), "invalid = {{").is_err());
        }
    }
}
