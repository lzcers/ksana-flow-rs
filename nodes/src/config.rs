use core::str;
use std::fs;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SourceConfig {
    pub tushare_token: String,
    pub db_uri: String,
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
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}
