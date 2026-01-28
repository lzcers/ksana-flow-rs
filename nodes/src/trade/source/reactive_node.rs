use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use flow::observable::ObservableExt;
use flow::{Context, Node, NodeInputs, OutputPayload, ReactiveStream};

use crate::trade::{
    k::K,
    source::{source::Source, tushare::Product},
    utils::date_to_str,
};

pub struct ReactiveSourceNode {
    source: Source,
    code: String,
    start_time: DateTime<Local>,
    end_time: Option<DateTime<Local>>,
}

impl ReactiveSourceNode {
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
        })
    }
}

#[async_trait]
impl Node for ReactiveSourceNode {
    async fn run(
        &mut self,
        _ctx: &Context,
        _inputs: NodeInputs,
    ) -> Result<OutputPayload, String> {
        let (code, start, end) = (self.code.clone(), self.start_time, self.end_time);
        let end_date = end
            .map(|d| d.date_naive())
            .unwrap_or_else(|| Local::now().date_naive());

        let mut result = self
            .source
            .get_daily_base_data(
                Some(Product::FUND),
                &code,
                (&date_to_str(&start.date_naive()), &date_to_str(&end_date)),
            )
            .await
            .unwrap_or_default();

        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let filtered_data = result
            .filter(|k| k.volume > 0.0)
            .delay(Duration::from_secs(5));

        // Convert Observable to ReactiveStream
        let stream = ReactiveStream::from_observable(filtered_data);
        Ok(OutputPayload::stream(stream.subscribe))
    }
}
