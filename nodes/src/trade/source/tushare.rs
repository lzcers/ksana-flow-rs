use crate::trade::{k::K, utils::date_str_to_timestamp};
use anyhow::Result;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;

pub enum Product {
    STOCK,
    FUND,
    INDEX,
}
pub struct TushareSource {
    /// Internal string holds tushare webapi access token.
    /// Used in every call as a hidden parameter.
    pub token: String,
    /// This is actually a constant of "http://api.tushare.pro"
    pub api_endpoint: String,
}

/// Tushare struct methods implementation
impl TushareSource {
    /// Only entry to create a tushare object
    /// # token
    /// The token is necessary for every call
    /// Apply it before you do any access
    pub fn new(token: &str) -> Self {
        TushareSource {
            token: token.to_string(),
            api_endpoint: "http://api.tushare.pro".to_string(),
        }
    }

    /// Create a QueryBuilder to actually build and process the query
    /// # api_name:
    pub fn querybuilder(self: &Self, api_name: &str) -> QueryBuilder {
        QueryBuilder::new(self, api_name)
    }

    pub async fn query_range_trade_date(&self, start: &str, end: &str) -> Result<Vec<String>> {
        let date_lsit = self
            .querybuilder("trade_cal")
            .addparam("start_date", start)
            .addparam("end_date", end)
            .addparam("is_open", "1")
            .fields("cal_date,is_open,pretrade_date") //optional step
            .query()
            .await?;
        Ok(date_lsit
            .iter()
            .map(|item| {
                item["cal_date"]
                    .as_str()
                    .expect("calc date parse failed.")
                    .to_owned()
            })
            .collect::<Vec<String>>())
    }

    pub async fn query_daily_base_data(
        &self,
        code: &str,
        product: Option<Product>,
        start: &str,
        end: &str,
    ) -> Result<Vec<K>> {
        let product = if let Some(product) = product {
            match product {
                Product::STOCK => "daily",
                Product::FUND => "fund_daily",
                Product::INDEX => "index_daily",
            }
        } else {
            "daily"
        };

        let result = self
            .querybuilder(product)
            .addparam("ts_code", code) //optional step
            .addparam("start_date", start) //opiontal step
            .addparam("end_date", end) //opiontal step
            .fields("ts_code,trade_date,open,high,low,close,vol,amount")
            .query()
            .await?
            .iter()
            .rev()
            .map(|v| K {
                code: v["ts_code"]
                    .as_str()
                    .expect("ts_code field error")
                    .to_owned(),
                timestamp: date_str_to_timestamp(
                    v["trade_date"].as_str().expect("trade_date field error"),
                ) as u64,
                open: v["open"].as_f64().expect("open field error"),
                high: v["high"].as_f64().expect("high field error"),
                low: v["low"].as_f64().expect("low field error"),
                close: v["close"].as_f64().expect("close field error"),
                volume: v["vol"].as_f64().expect("vol field error"),
                amount: v["amount"].as_f64().expect("amount field error"),
            })
            .collect::<Vec<K>>();
        Ok(result)
    }
}

/// TushareError enumerates all possible errors returned by this library.
#[derive(Error, Debug)]
pub enum TushareError {
    /// Tushare returns empty rows.
    /// It might have returned dataframe column names but it's impossible to infer column type without row data
    /// If this is the intended behavior, the caller should handle this error  
    #[error("Tushare returned empty data")]
    EmptyError,
    /// Tushare returns non-zero error code in response body
    #[error("Tushare request return error:{code}, msg:{msg}")]
    RequestError { code: String, msg: String },
    /// Transform Tushare returned json to polars json error
    #[error("Expected json node {0} not exist")]
    DataError(String),

    /// Represents a network failure to read tushare web api.
    #[error("Request network error, not accessable or possible 500")]
    NetworkError(#[from] reqwest::Error),

    /// Represents a failure to decode tushare result json
    #[error("Parse tushare response json error")]
    JsonError(#[from] serde_json::Error),
}

/// Used to specify API parameter pairs
pub type Dict = HashMap<String, String>;

fn mergedict(map_pre: Dict, map_post: Dict) -> Dict {
    map_pre.into_iter().chain(map_post).collect()
}

/// A tushare query that satistfies rust builder pattern.
/// The QueryBuilder is immutable, which means a new instance
/// of QueryBuilder will be created during params()/addparam()/fields() calling.
/// So it is safe for multi-threading
pub struct QueryBuilder<'a> {
    tushare: &'a TushareSource,
    api_name: String,
    params: Option<Dict>,
    fields: Option<String>,
}

impl<'a> QueryBuilder<'a> {
    pub(crate) fn new(tushare: &'a TushareSource, api_name: &str) -> Self {
        QueryBuilder {
            tushare,
            api_name: api_name.to_string(),
            params: None,
            fields: None,
        }
    }

    /// Set parameters to the query. Parameters are e.g. trade_date, start_date, end_date, market, exchange.
    /// For detailed param explanation, see the tushare api website <https://tushare.pro/document/2?doc_id=25> .
    /// Note this step is optional, you can safely ignore this during ramp up, and the return will be up to 6,000 rows.
    /// The main purpose of parameters is to define your requirements clearly
    /// # param
    /// The predefined request parameters according to each api_name, e.g. 'start_date', 'end_date'
    pub fn params(self: &Self, params: Dict) -> Self {
        QueryBuilder {
            tushare: self.tushare,
            api_name: self.api_name.clone(),
            params: Some(params),
            fields: self.fields.clone(),
        }
    }

    /// Add a parameter to the query, e.g. trade_date, start_date, end_date, market, exchange.
    /// This is a helper function for params() since constructing a hashmap is a little bit boring.
    /// Parameter pairs with the same key will be overwritten.
    /// For detailed param explanation, see the tushare api website <https://tushare.pro/document/2?doc_id=25> .
    /// Note this is optional, you can ignore this during ramp up, and the return will be up to 6,000 rows.
    /// The main purpose of parameters is to define your requirements clearly.
    /// # k/v
    /// The predefined request key/value pair according to each api_name, e.g. 'start_date', 'end_date'
    pub fn addparam(self: &Self, k: &str, v: &str) -> Self {
        let new_paramdict = Dict::from([(k.to_string(), v.to_string())]);
        let paramdict = match &self.params {
            Some(dict) => mergedict(dict.clone(), new_paramdict),
            None => new_paramdict,
        };
        QueryBuilder {
            tushare: self.tushare,
            api_name: self.api_name.clone(),
            params: Some(paramdict),
            fields: self.fields.clone(),
        }
    }
    /// Set the return fields to the query.
    /// For detailed return field explanation, see the tushare api website https://tushare.pro/document/2?doc_id=25 .
    /// Note this is optional, you can ignore this during ramp up, and the return will be up to 10~20 columns.
    /// You may want to use it to reduce network IO and clarify your requirement clearly.
    /// # fields
    /// The predefined fields string separated with commas, e.g. "ts_code,trade_date,open,high,low,close,pre_close"
    pub fn fields(self: &Self, fields: &str) -> Self {
        QueryBuilder {
            tushare: self.tushare,
            api_name: self.api_name.clone(),
            params: self.params.clone(),
            fields: Some(fields.to_string()),
        }
    }

    fn build(self: &Self) -> Value {
        match (&self.params, &self.fields) {
            (Some(p), Some(f)) => json!({
                "api_name":self.api_name,
                "token":self.tushare.token,
                "params": p,
                "fields": f
            }),
            (Some(p), None) => json!({
                "api_name":self.api_name,
                "token":self.tushare.token,
                "params": p,
                "fields": null
            }),
            (None, Some(f)) => json!({
                "api_name":self.api_name,
                "token":self.tushare.token,
                "params": null,
                "fields": f
            }),
            (None, None) => json!({
                "api_name":self.api_name,
                "token":self.tushare.token,
                "params": null,
                "fields": null
            }),
        }
    }

    fn json_reformat(resp_json: Value) -> Result<Vec<Value>, TushareError> {
        let mut data_json: Vec<Value> = vec![];
        let fields_json = resp_json["data"]["fields"]
            .as_array()
            .ok_or(TushareError::DataError("data/fields".to_string()))?;
        let mut fields: Vec<&str> = vec![];
        for (i, field) in fields_json.iter().enumerate() {
            let _field = field
                .as_str()
                .ok_or(TushareError::DataError(format!("data/fields at {i}")))?;
            fields.push(_field);
        }
        let data = resp_json["data"]["items"]
            .as_array()
            .ok_or(TushareError::DataError("data/items".to_string()))?;
        for (i, item) in data.iter().enumerate() {
            let item_data = item.as_array().ok_or(TushareError::DataError(format!(
                "data/items/{i} is expected to be an array"
            )))?;
            let mut item_json: serde_json::Map<String, Value> = serde_json::Map::new();
            for (k, v) in fields.iter().zip(item_data.iter()) {
                item_json.insert(k.to_string(), v.clone());
            }
            data_json.push(Value::Object(item_json))
        }
        Ok(data_json)
    }

    /// Query API predefined request type & parameters and return a Data Frame as output
    /// Fundamental entry for every tushare data access.
    pub async fn query(self: &Self) -> Result<Vec<Value>, TushareError> {
        let tushare_request = self.build();
        info!(
            "Request text:\n {}\n",
            serde_json::to_string(&tushare_request).unwrap_or("to str error".to_string())
        );

        let client = Client::new();
        let resp_text = client
            .post(self.tushare.api_endpoint.clone())
            .body(tushare_request.to_string())
            .send()
            .await? // sending network error
            .error_for_status()? // 400 or other http error
            .text()
            .await?;
        let resp_json: Value = serde_json::from_str(&resp_text)?;
        if let Some(ret_code) = resp_json["code"].as_i64() {
            info!("resp code: {:?}", ret_code);
            if ret_code != 0 {
                let code = resp_json["code"].as_str().unwrap_or("unknown");
                let msg = resp_json["msg"].as_str().unwrap_or("unknown");
                return Err(TushareError::RequestError {
                    code: code.to_string(),
                    msg: msg.to_string(),
                });
            }
        }
        let data_json = Self::json_reformat(resp_json)?;
        let data_str = serde_json::to_string(&data_json)?;
        if data_str == "" || data_str == "[]" {
            return Err(TushareError::EmptyError);
        }
        Ok(data_json)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::get_config;

    use super::*;

    fn init() -> Result<TushareSource> {
        let config = get_config()?;
        Ok(TushareSource::new(&config.source.tushare_token))
    }

    #[tokio::test]
    async fn query_daily_stock_base_bar() -> Result<()> {
        let source = init()?;
        let list = source
            .query_daily_base_data("399300.SZ", None, "20240901", "20241001")
            .await?;
        for bar in list {
            println!("{:?}", bar);
        }
        // println!("{:?}", list.len());
        Ok(())
    }

    #[tokio::test]
    async fn get_range_trade_date() -> Result<()> {
        let source = init()?;
        let list = source
            .query_range_trade_date("20240901", "20241001")
            .await?;
        println!("{list:?}");
        Ok(())
    }
}
