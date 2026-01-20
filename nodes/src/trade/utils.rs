use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use csv::{Writer, WriterBuilder};
use std::fs::File;
use std::fs::OpenOptions;

pub fn create_write_csv(file_path: &str, _header: Option<Vec<&str>>) -> Result<Writer<File>> {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    // 创建 CSV writer 并设置为不写入头部
    let wtr = WriterBuilder::new().has_headers(false).from_writer(file);
    Ok(wtr)
}

pub fn date_str_to_timestamp(date_str: &str) -> i64 {
    // 解析日期字符串
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y%m%d").expect("Failed to parse date");
    // 将日期转换为 NaiveDateTime，假设时间为 00:00:00
    let naive_date_time = NaiveDateTime::new(naive_date, NaiveTime::default());
    // 将 NaiveDateTime 转换为 UTC 时间
    let utc_date_time = Utc.from_utc_datetime(&naive_date_time).fixed_offset();
    // 获取时间戳
    let timestamp = utc_date_time.timestamp_millis();
    timestamp
}

pub fn date_to_str(date: &NaiveDate) -> String {
    date.format("%Y%m%d").to_string()
}

pub fn timestamp_to_str(timestamp: u64) -> String {
    let datetime = DateTime::from_timestamp_millis(timestamp as i64).unwrap();
    datetime.date_naive().to_string()
}
