use super::utils::timestamp_to_str;
use serde::{Deserialize, Serialize};
pub use ta::{Close, High, Low, Open, Volume};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct K {
    pub code: String,
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub amount: f64,
}

impl K {
    pub fn new(code: String) -> Self {
        Self {
            code,
            timestamp: 0,
            open: 0.0,
            close: 0.0,
            low: 0.0,
            high: 0.0,
            volume: 0.0,
            amount: 0.0,
        }
    }
}

impl std::fmt::Display for K {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {} {}",
            self.code,
            timestamp_to_str(self.timestamp),
            self.open,
            self.close,
            self.low,
            self.high,
            self.volume,
            self.amount,
        )
    }
}

impl Open for K {
    fn open(&self) -> f64 {
        self.open
    }
}

impl Close for K {
    fn close(&self) -> f64 {
        self.close
    }
}

impl Low for K {
    fn low(&self) -> f64 {
        self.low
    }
}

impl High for K {
    fn high(&self) -> f64 {
        self.high
    }
}

impl Volume for K {
    fn volume(&self) -> f64 {
        self.volume
    }
}
