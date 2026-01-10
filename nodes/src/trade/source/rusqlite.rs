use anyhow::Result;
use rusqlite::{Connection, params};
use std::sync::Mutex;
use tracing::info;

use crate::{config::get_config, trade::k::K};

/// 持久化存储股票数据
pub struct SqliteSource {
    db_url: String,
    conn: Mutex<Connection>,
}

impl SqliteSource {
    pub fn new() -> Result<Self> {
        let config = get_config()?;
        let db_url = config.source.db_uri;
        let conn = Connection::open(&db_url)?;
        return Ok(SqliteSource {
            db_url,
            conn: Mutex::new(conn),
        });
    }
    pub fn init(&self) -> Result<()> {
        // 创建股票基础信息表
        self.conn.lock().unwrap().execute(
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
        self.conn.lock().unwrap().execute(
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
        self.conn.lock().unwrap().execute(
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
        self.conn.lock().unwrap().execute(
            "INSERT OR IGNORE INTO daily_stock_base_data (code, timestamp, open, high, low, close, volume, amount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![data.code, data.timestamp, data.open, data.high, data.low, data.close, data.volume, data.amount],
        )?;
        Ok(())
    }

    pub fn insert_live_stock_base_data(&self, data: &K, current_price: f64) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT OR IGNORE live_stock_base_data (code, timestamp, current, open, high, low, close, volume, amount) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![data.code, data.timestamp, current_price, data.open, data.high, data.low, data.close, data.volume, data.amount],
        )?;
        Ok(())
    }

    pub fn insert_trading_date(&self, date: u32) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT OR IGNORE INTO trading_date (date) VALUES (?1)",
            params![date],
        )?;
        Ok(())
    }

    pub fn is_trading_date(&self, date: u32) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT count(*) FROM trading_date WHERE date = ?1")?;
        let count: i32 = stmt.query_row(params![date], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn get_all_trading_date(&self, date_range: (u32, u32)) -> Result<Vec<u32>> {
        let (start, end) = date_range;
        let query_sql = format!(
            "SELECT date FROM trading_date WHERE date >= {} AND date <= {} ORDER BY date ASC",
            start, end
        );
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&query_sql)?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn has_daily_stock_data(&self, code: &str, timestamp: Option<u64>) -> Result<bool> {
        let mut sqlstr = format!(
            "SELECT count(*) FROM daily_stock_base_data WHERE code = '{}'",
            code
        );
        if let Some(timestamp) = timestamp {
            sqlstr.push_str(&format!(" AND timestamp = {}", timestamp));
        }
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sqlstr)?;
        let count: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn get_daily_stock_base_data(
        &self,
        code: &str,
        timestamp_range: Option<(u64, u64)>,
    ) -> Result<Vec<K>> {
        let mut query_sql = format!(
            "SELECT code, timestamp, open, high, low, close, volume, amount FROM daily_stock_base_data WHERE code = '{}'",
            code
        );
        if let Some((start, end)) = timestamp_range {
            query_sql.push_str(&format!(
                " AND timestamp >= {} AND timestamp <= {}",
                start, end
            ));
        }
        query_sql.push_str(" ORDER BY timestamp ASC");

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&query_sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(K {
                code: row.get(0)?,
                timestamp: row.get(1)?,
                open: row.get(2)?,
                high: row.get(3)?,
                low: row.get(4)?,
                close: row.get(5)?,
                volume: row.get(6)?,
                amount: row.get(7)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
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

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&query_sql)?;
        info!("Query Sqlite statement is: {query_sql}");

        let mut stock_list = stmt.query(params![code])?;
        while let Some(row) = stock_list.next()? {
            query_result.push(K {
                code: row.get::<_, String>(0)?,
                timestamp: row.get::<_, u64>(1)?,
                open: row.get::<_, f64>(3)?,
                high: row.get::<_, f64>(4)?,
                low: row.get::<_, f64>(5)?,
                close: row.get::<_, f64>(6)?,
                volume: row.get::<_, f64>(7)?,
                amount: row.get::<_, f64>(8)?,
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
