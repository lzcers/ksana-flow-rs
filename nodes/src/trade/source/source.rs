#![allow(dead_code)]
use super::rusqlite::SqliteSource;
use super::tushare::{Product, TushareSource};
use super::xueqiu::XueqiuSource;
use crate::config::get_config;
use crate::trade::{k::K, utils::date_str_to_timestamp};
use anyhow::{Error, Result};

use tracing::info;

pub enum SourceError {
    EmptyError,
}

/// 数据源是一个根据定时任务触发的变动数据状态
pub struct Source {
    pub(crate) sqlite: SqliteSource,
    xueqiu: XueqiuSource,
    tushare: TushareSource,
}

impl Source {
    pub fn new() -> Result<Self> {
        let config = get_config()?;
        let xueqiu = XueqiuSource::new()?;
        let sqlite = SqliteSource::new()?;
        let tushare = TushareSource::new(&config.source.tushare_token);
        sqlite.init()?;
        Ok(Source {
            xueqiu,
            sqlite,
            tushare,
        })
    }

    pub fn get_all_trading_date(&self, date_range: (&str, &str)) -> Result<Vec<u32>> {
        let (start, end) = date_range;
        let trading_date_list = self
            .sqlite
            .get_all_trading_date((start.parse::<u32>()?, end.parse::<u32>()?))?;
        Ok(trading_date_list)
    }

    pub async fn get_daily_base_data(
        &self,
        product: Option<Product>,
        code: &str,
        date_range: (&str, &str),
    ) -> Result<Vec<K>> {
        let (start, end) = date_range;
        if self.sqlite.has_daily_stock_data(code, None)? {
            let trading_date_list = self
                .sqlite
                .get_all_trading_date((start.parse::<u32>()?, end.parse::<u32>()?))?;

            let local_result = self.sqlite.get_daily_stock_base_data(
                code,
                Some((
                    date_str_to_timestamp(start) as u64,
                    date_str_to_timestamp(end) as u64,
                )),
            )?;
            info!(
                "Source Query: local data len is {}, trading_date_num is {}",
                local_result.len(),
                trading_date_list.len(),
            );
            if trading_date_list.len() == local_result.len() {
                info!("Source Query: data from sqlite {code} {start}-{end}");
                return Ok(local_result);
            }
        }

        // query and insert from tushare
        let list = self
            .tushare
            .query_daily_base_data(code, product, start, end)
            .await?;
        info!(
            "Source Query: data from tushare {code} {start}-{end}, list len: {}",
            list.len()
        );
        for bar in list.iter() {
            self.sqlite.insert_daily_stock_base_data(bar)?;
        }
        Ok(list)
    }

    pub async fn get_live_base_data(&self, code: &str) -> Result<(K, f64)> {
        let xueqiu_code = if code.contains(".") {
            let code_slice = code.split(".").collect::<Vec<&str>>();
            code_slice.last().unwrap_or(&"Code parse error").to_string()
                + code_slice.first().unwrap_or(&"Code parse error")
        } else {
            code.to_string()
        };
        let live_result = self.xueqiu.get_realtime_bar(&xueqiu_code).await?;
        if let Some(bar) = live_result.data.into_iter().next() {
            let live_bar = K {
                code: xueqiu_code,
                timestamp: bar.timestamp,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.last_close,
                volume: (bar.volume / 100) as f64,
                amount: bar.amount,
            };
            return Ok((live_bar, bar.current));
        }
        Err(Error::msg("get live data failed."))
    }

    pub async fn update_trading_date(&self, start: &str, end: &str) -> Result<()> {
        let trading_date_list = self.tushare.query_range_trade_date(start, end).await?;
        for sdate in trading_date_list {
            let udate = sdate.parse::<u32>()?;
            self.sqlite.insert_trading_date(udate)?;
        }
        Ok(())
    }
}
