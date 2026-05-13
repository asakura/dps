//! Platform-aware configuration and data directory resolution.

use std::path::PathBuf;

use directories::ProjectDirs;

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
    let config_env = format!("{}_CONFIG", env!("CARGO_PKG_NAME").to_uppercase());
    if let Ok(s) = std::env::var(config_env) {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".config")
    }
}

/// Returns the data directory used for logs and application state.
///
/// Resolution order:
/// 1. `DPS_DATA` environment variable
/// 2. Platform data directory (`~/.local/share/dps` on Linux)
/// 3. `.data` in the current working directory
pub fn get_data_dir() -> PathBuf {
    let data_env = format!("{}_DATA", env!("CARGO_PKG_NAME").to_uppercase());
    if let Ok(s) = std::env::var(data_env) {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    mod get_config_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            unsafe { env::set_var("DPS_CONFIG", "/tmp/dps-test-config") };
            let dir = get_config_dir();
            unsafe { env::remove_var("DPS_CONFIG") };
            assert_eq!(dir, PathBuf::from("/tmp/dps-test-config"));
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            unsafe { env::remove_var("DPS_CONFIG") };
            assert!(!get_config_dir().as_os_str().is_empty());
        }
    }

    mod get_data_dir_fn {
        use super::*;

        #[test]
        fn env_var_overrides_platform_dir() {
            unsafe { env::set_var("DPS_DATA", "/tmp/dps-test-data") };
            let dir = get_data_dir();
            unsafe { env::remove_var("DPS_DATA") };
            assert_eq!(dir, PathBuf::from("/tmp/dps-test-data"));
        }

        #[test]
        fn returns_nonempty_path_without_env_var() {
            unsafe { env::remove_var("DPS_DATA") };
            assert!(!get_data_dir().as_os_str().is_empty());
        }
    }
}
