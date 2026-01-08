use anyhow::Result;
use rusqlite::{Connection, params};
use tracing::info;

use crate::trade::{config::get_config, k::K};

/// 持久化存储股票数据
pub struct SqliteSource {
    db_url: String,
    conn: Connection,
}

impl SqliteSource {
    pub fn new() -> Result<Self> {
        let config = get_config()?;
        let db_url = config.source.db_uri;
        let conn = Connection::open(&db_url)?;
        return Ok(SqliteSource { db_url, conn });
    }
    pub fn init(&self) -> Result<()> {
        // 创建股票基础信息表
        self.conn.execute(
            r#"
                CREATE TABLE if not exists daily_stock_base_data (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    code TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    open REAL NOT NULL,
                    high REAL NOT NULL,
                    low REAL NOT NULL,
                    close REAL NOT NULL,
                    volume INTEGER NOT NULL,
                    amount REAL NOT NULL,
                    UNIQUE(code, timestamp)
                );
                "#,
            [],
        )?;

        // 实时数据表
        self.conn.execute(
            r#"
                CREATE TABLE if not exists live_stock_base_data (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    code TEXT NOT NULL,
                    timestamp INTEGER NOT NULL,
                    open REAL NOT NULL,
                    high REAL NOT NULL,
                    low REAL NOT NULL,
                    close REAL NOT NULL,
                    volume INTEGER NOT NULL,
                    amount REAL NOT NULL,
                    UNIQUE(code, timestamp)
                );
                "#,
            [],
        )?;

        // 交易日历表
        self.conn.execute(
            r#"
                CREATE TABLE if not exists trading_date (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    date INTEGER NOT NULL,
                    UNIQUE(date)
                );
                "#,
            [],
        )?;

        Ok(())
    }

    pub fn insert_daily_stock_base_data(&self, data: &K) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO daily_stock_base_data (code, timestamp, open, high, low, close, volume, amount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![data.code, data.timestamp, data.open, data.high, data.low, data.close, data.volume, data.amount],
        )?;
        Ok(())
    }

    pub fn insert_live_stock_base_data(&self, data: &K, current_price: f64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE live_stock_base_data (code, timestamp, current, open, high, low, close, volume, amount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![data.code, data.timestamp, current_price, data.open, data.high, data.low, data.close, data.volume, data.amount],
        )?;
        Ok(())
    }

    pub fn insert_trading_date(&self, date: u32) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO trading_date (date) VALUES (?1)",
            params![date],
        )?;
        Ok(())
    }

    pub fn is_trading_date(&self, date: u32) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT EXISTS(SELECT 1 FROM trading_date WHERE date = ?1)")?;
        Ok(stmt.query_row(params![date], |row| row.get(0))?)
    }

    pub fn get_all_trading_date(&self, range: (u32, u32)) -> Result<Vec<u32>> {
        let query_sql = r#"
                    SELECT date FROM trading_date
                    WHERE date BETWEEN ?1 AND ?2
                    "#;
        let mut query_result = vec![];
        let mut stmt = self.conn.prepare(query_sql)?;
        let mut result_row = stmt.query(params![range.0, range.1])?;

        while let Some(row) = result_row.next()? {
            let date: u32 = row.get(0)?;
            query_result.push(date);
        }
        Ok(query_result)
    }

    pub fn has_daily_stock_data(&self, code: &str, timestamp: Option<i64>) -> Result<bool> {
        // 使用 SELECT EXISTS 查询是否存在指定的 code 值
        let mut sqlstr =
            "SELECT EXISTS(SELECT 1 FROM daily_stock_base_data WHERE code = ?1)".to_string();
        if let Some(date) = timestamp {
            sqlstr.push_str(&format!("AND timestamp = {date}"));
        }
        let mut stmt = self.conn.prepare(&sqlstr)?;
        Ok(stmt.query_row(params![code], |row| row.get(0))?)
    }

    pub fn get_daily_stock_base_data(
        &self,
        code: &str,
        date_range: Option<(i64, i64)>,
    ) -> Result<Vec<K>> {
        let mut query_result = vec![];
        let mut query_sql = r#"
                    SELECT code, timestamp, open, high, low, close, volume, amount FROM daily_stock_base_data
                    WHERE code = ?1
                    "#.to_string();
        if let Some((start, end)) = date_range {
            query_sql.push_str(&format!("AND timestamp BETWEEN {start} AND {end} "))
        }
        query_sql.push_str("ORDER BY timestamp ASC;");
        let mut stmt = self.conn.prepare(&query_sql)?;
        info!("Query Sqlite statement is: {query_sql}");

        let mut stock_list = stmt.query(params![code])?;
        while let Some(row) = stock_list.next()? {
            query_result.push(K {
                code: row.get(0)?,
                timestamp: row.get(1)?,
                open: row.get(2)?,
                high: row.get(3)?,
                low: row.get(4)?,
                close: row.get(5)?,
                volume: row.get(6)?,
                amount: row.get(7)?,
            });
        }
        Ok(query_result)
    }

    pub fn get_live_stock_base_data(
        &self,
        code: &str,
        date_range: Option<(&str, &str)>,
    ) -> Result<Vec<K>> {
        let mut query_result = vec![];
        let mut query_sql = r#"
                    SELECT code, timestamp, current, open, high, low, close, volume, amount FROM live_stock_base_data
                    WHERE code = ?1
                    "#.to_string();
        if let Some((start, end)) = date_range {
            query_sql.push_str(&format!("AND timestamp BETWEEN {start} AND {end}"))
        }

        let mut stmt = self.conn.prepare(&query_sql)?;
        info!("Query Sqlite statement is: {query_sql}");

        let mut stock_list = stmt.query(params![code])?;
        while let Some(row) = stock_list.next()? {
            query_result.push(K {
                code: row.get(0)?,
                timestamp: row.get(1)?,
                open: row.get(3)?,
                high: row.get(4)?,
                low: row.get(5)?,
                close: row.get(6)?,
                volume: row.get(7)?,
                amount: row.get(8)?,
            });
        }
        Ok(query_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() -> Result<SqliteSource> {
        let sqlite = SqliteSource::new()?;
        sqlite.init()?;
        Ok(sqlite)
    }
    #[test]
    fn test_has_daily_base_data_false() -> Result<()> {
        let source: SqliteSource = init()?;
        let r = source.has_daily_stock_data("1234567.SZ", None)?;
        assert_eq!(false, r);
        Ok(())
    }

    #[test]
    fn test_has_daily_base_data_true() -> Result<()> {
        let source: SqliteSource = init()?;
        let r = source.has_daily_stock_data("399300.SZ", None)?;
        assert_eq!(true, r);
        Ok(())
    }
}
