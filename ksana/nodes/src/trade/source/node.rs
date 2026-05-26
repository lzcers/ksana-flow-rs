use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use flow::{Context, Input, Node, Output};
use serde_json::Value;

use crate::trade::{
    k::K,
    source::{source::Source, tushare::Product},
    utils::date_to_str,
};

pub struct SourceNode {
    source: Source,
    code: String,
    start_time: DateTime<Local>,
    end_time: Option<DateTime<Local>>,
    cached_data: Option<Vec<K>>,
    cursor: usize,
}

impl SourceNode {
    pub fn new(
        code: &str,
        start_time: DateTime<Local>,
        end_time: Option<DateTime<Local>>,
    ) -> Result<Self> {
        Ok(Self {
            source: Source::new()?,
            code: code.to_owned(),
            start_time,
            end_time,
            cached_data: None,
            cursor: 0,
        })
    }
}

impl SourceNode {
    async fn ensure_data(&mut self) -> Result<()> {
        if self.cached_data.is_none() {
            let (code, start, end) = (self.code.clone(), self.start_time, self.end_time);
            let end_date = end
                .map(|d| d.date_naive())
                .unwrap_or_else(|| Local::now().date_naive());

            let result = self
                .source
                .get_daily_base_data(
                    Some(Product::FUND),
                    &code,
                    (&date_to_str(&start.date_naive()), &date_to_str(&end_date)),
                )
                .await?;

            // 策略计算通常需要按时间正序排列
            let mut data = result;
            data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            self.cached_data = Some(data);
            self.cursor = 0;
        }
        Ok(())
    }

    pub async fn get_range_data(&mut self) -> Result<Vec<K>> {
        self.ensure_data().await?;
        Ok(self.cached_data.clone().unwrap_or_default())
    }
}

#[async_trait]
impl Node for SourceNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        if self.ensure_data().await.is_err() {
            return Ok(Value::Null.into());
        }

        if let Some(ref data) = self.cached_data {
            if self.cursor < data.len() {
                let k = data[self.cursor].clone();
                self.cursor += 1;
                return Ok(serde_json::to_value(k).unwrap_or(Value::Null).into());
            }
        }
        Ok(Value::Null.into())
    }
}
