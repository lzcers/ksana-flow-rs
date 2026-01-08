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
pub struct LogConfig {
    pub output_dir: String,
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
    pub log: LogConfig,
    pub email: EmailConfig,
}

pub fn get_config() -> Result<Config> {
    let config_content = fs::read_to_string("./config.toml")?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}
