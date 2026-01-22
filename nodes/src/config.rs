use core::str;
use std::fs;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SourceConfig {
    pub tushare_token: String,
    pub db_uri: String,
    pub data_db_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    pub email: String,
    pub email_token: String,
    pub smtp_server: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub source: SourceConfig,
    pub email: EmailConfig,
}

pub fn get_config() -> Result<Config> {
    // 1. Try finding config.toml in current working directory (Runtime)
    let cwd_config = std::path::Path::new("config.toml");
    
    // 2. Fallback to CARGO_MANIFEST_DIR (Development)
    let config_path = if cwd_config.exists() {
        cwd_config.to_path_buf()
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("config.toml")
    };

    if !config_path.exists() {
        return Err(anyhow::anyhow!("config.toml not found at {:?}", config_path));
    }

    let config_content = fs::read_to_string(&config_path)?;
    let mut config: Config = toml::from_str(&config_content)?;

    // resolve db_uri to absolute path
    // If we found config in CWD, resolve relative to CWD
    // If we found config in MANIFEST_DIR, resolve relative to project root (parent of nodes)
    let base_dir = if cwd_config.exists() {
        std::env::current_dir()?
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    };

    let db_path = std::path::Path::new(&config.source.db_uri);
    if db_path.is_relative() {
        let abs_path = base_dir.join(&config.source.db_uri);
        let data_db_abs_path = base_dir.join(&config.source.data_db_uri);
        config.source.db_uri = abs_path.to_string_lossy().to_string();
        config.source.data_db_uri = data_db_abs_path.to_string_lossy().to_string();
    }

    Ok(config)
}
