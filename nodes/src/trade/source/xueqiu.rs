#![allow(dead_code)]
use anyhow::Result;
use reqwest::Client;
use reqwest::header::{ACCEPT_ENCODING, HeaderMap};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Data {
    pub symbol: String,
    pub current: f64,
    pub percent: f64,
    pub chg: f64,
    pub timestamp: i64,
    pub volume: u64,
    pub amount: f64,
    pub turnover_rate: Option<f64>,
    pub amplitude: f64,
    pub last_close: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub avg_price: f64,
    pub current_year_percent: f64,
}

#[derive(Deserialize, Debug)]
pub struct XuequiRealtimeApiResponse {
    pub data: Vec<Data>,
    pub error_code: i32,
    pub error_description: Option<String>,
}

/// 雪球 HTTP API 接口获取实时数据
pub struct XueqiuSource {
    realtime_api_url: String,
    client: Client,
}

impl XueqiuSource {
    pub fn new() -> Result<XueqiuSource> {
        let realtime_api_url =
            "https://stock.xueqiu.com/v5/stock/realtime/quotec.json?symbol=".to_string();
        let mut default_headers = HeaderMap::new();
        default_headers.insert(ACCEPT_ENCODING, "gzip".parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .build()?;

        Ok(XueqiuSource {
            realtime_api_url,
            client,
        })
    }

    pub async fn get_realtime_bar(&self, code: &str) -> Result<XuequiRealtimeApiResponse> {
        let query_url = format!("{}{code}", &self.realtime_api_url);
        let result = self
            .client
            .get(&query_url)
            .send()
            .await?
            .json::<XuequiRealtimeApiResponse>()
            .await?;

        Ok(result)
    }
}

#[tokio::test]
async fn test_get_realtime_bar() -> Result<()> {
    let req = XueqiuSource::new()?;
    let result = req.get_realtime_bar("SH510300").await;
    println!("{:?}", result);
    Ok(())
}
