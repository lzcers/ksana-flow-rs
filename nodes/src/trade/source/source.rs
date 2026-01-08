use super::rusqlite::SqliteSource;
use super::tushare::Product;
use super::tushare::TushareSource;
use super::xueqiu::XueqiuSource;
use crate::trade::{k::K, utils::date_str_to_timestamp};
use anyhow::{Error, Result};
use tracing::info;

pub enum SourceError {
    EmptyError,
}

/// 数据源是一个根据定时任务触发的变动数据状态
pub struct Source {
    xueqiu: XueqiuSource,
    sqlite: SqliteSource,
    tushare: TushareSource,
}

impl Source {
    pub fn new(tushare_token: &str) -> Result<Self> {
        let xueqiu = XueqiuSource::new()?;
        let sqlite = SqliteSource::new()?;
        let tushare = TushareSource::new(tushare_token);
        sqlite.init()?;
        Ok(Source {
            xueqiu,
            sqlite,
            tushare,
        })
    }

    pub fn get_range_trade_date(start: &str, end: &str) {}

    pub fn get_daily_base_data(
        &self,
        product: Option<Product>,
        code: &str,
        date_range: (&str, &str),
    ) -> Result<Vec<K>> {
        let query_and_insert_from_tushare = || {
            let (start, end) = date_range;
            let list = self
                .tushare
                .query_daily_base_data(code, product, start, end)?;
            info!(
                "Source Query: data from tushare {code} {start}-{end}, list len: {}",
                list.len()
            );
            for bar in list.iter() {
                self.sqlite.insert_daily_stock_base_data(bar)?;
            }
            Ok(list)
        };
        if self.sqlite.has_daily_stock_data(code, None)? {
            let (start, end) = date_range;
            let trading_date_list = self
                .sqlite
                .get_all_trading_date((start.parse::<u32>()?, end.parse::<u32>()?))?;

            let local_result = self.sqlite.get_daily_stock_base_data(
                code,
                Some((date_str_to_timestamp(start), date_str_to_timestamp(end))),
            )?;
            info!(
                "Source Query: local data len is {}, trading_date_num is {}",
                local_result.len(),
                trading_date_list.len(),
            );
            if trading_date_list.len() == local_result.len() {
                info!("Source Query: data from sqlite {code} {start}-{end}");
                return Ok(local_result);
            } else {
                query_and_insert_from_tushare()
            }
        } else {
            query_and_insert_from_tushare()
        }
    }

    pub fn get_live_base_data(&self, code: &str) -> Result<(K, f64)> {
        let xueqiu_code = if code.contains(".") {
            let code_slice = code.split(".").collect::<Vec<&str>>();
            code_slice.last().unwrap_or(&"Code parse error").to_string()
                + code_slice.first().unwrap_or(&"Code parse error")
        } else {
            code.to_string()
        };
        let live_result = self.xueqiu.get_realtime_bar(&xueqiu_code)?;
        for bar in live_result.data {
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

    pub fn update_trading_date(&self, start: &str, end: &str) -> Result<()> {
        let trading_date_list = self.tushare.query_range_trade_date(start, end)?;
        for sdate in trading_date_list {
            let udate = sdate.parse::<u32>()?;
            self.sqlite.insert_trading_date(udate)?;
        }
        Ok(())
    }

    // 更新一支股票的日数据
    pub fn update_daily_stock_base_data(&self, code: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::config::get_config;

    pub fn init() -> Result<Source> {
        let config = get_config()?;
        Ok(Source::new(&config.source.tushare_token)?)
    }

    #[test]
    fn test_update_trading_date() -> Result<()> {
        let source = init()?;
        source.update_trading_date("20210101", "20251230")?;
        Ok(())
    }

    #[test]
    fn test_get_trading_date() -> Result<()> {
        let source = init()?;
        let (start, end) = ("20210101", "20251230");

        let trading_date_list = source
            .sqlite
            .get_all_trading_date((start.parse::<u32>()?, end.parse::<u32>()?))?;
        println!("{:?}", trading_date_list.len());
        Ok(())
    }
}
