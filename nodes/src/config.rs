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
    let base_path = env!("CARGO_MANIFEST_DIR");
    let config_path = std::path::Path::new(base_path).join("config.toml");

    let config_content = fs::read_to_string(config_path)?;
    let mut config: Config = toml::from_str(&config_content)?;

    // resolve db_uri to absolute path
    let db_path = std::path::Path::new(&config.source.db_uri);
    if db_path.is_relative() {
        let project_root = std::path::Path::new(base_path).parent().unwrap();
        let abs_path = project_root.join(&config.source.db_uri);
        let data_db_abs_path = project_root.join(&config.source.data_db_uri);
        config.source.db_uri = abs_path.to_string_lossy().to_string();
        config.source.data_db_uri = data_db_abs_path.to_string_lossy().to_string();
    }

    Ok(config)
}
