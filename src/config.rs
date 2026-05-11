use std::path::PathBuf;

use directories::ProjectDirs;

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
}

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
