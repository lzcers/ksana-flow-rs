use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Order {
    BUY,  // 买单
    SELL, //卖单
    HOLD, // 持仓
}

pub struct OrderState {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrderInfo {
    pub id: u64,
    pub order: Order,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub code: String,

    /// 开仓均价
    pub open_price: f64,

    /// 持仓量
    pub quantity: f64,

    /// 收益。
    pub profit: f64,

    /// 收益率。
    pub profit_ratio: f64,

    /// 手续费。
    pub fee: f64,

    /// 开仓时间。
    pub open_time: u64,

    /// 平仓时间。
    pub close_time: Option<u64>,
}
// 交易接口

impl Position {
    // 开新仓
    pub fn open_position(code: &str, timestamp: u64, current_price: f64, quantity: f64) -> Self {
        Self {
            code: code.to_string(),
            open_price: current_price,
            quantity,
            profit: 0.0,
            profit_ratio: 0.0,
            fee: 0.0,
            open_time: timestamp,
            close_time: None,
        }
    }
    pub fn close_position(&mut self, timestamp: u64, current_price: f64) -> f64 {
        let income = self.sub_position(timestamp, current_price, self.quantity);
        self.close_time = Some(timestamp);
        income
    }

    pub fn add_position(&mut self, current_price: f64, quantity: f64) -> f64 {
        // 按照最新成交价购买的持仓成本
        let add_cost = current_price * quantity;
        let prev_cost = self.quantity * self.open_price;

        // 新的持仓均价 = (新增成本 + 旧仓成本) / 总数
        self.quantity += quantity;
        self.open_price = (add_cost + prev_cost) / self.quantity;
        self.update_position_profit(current_price);
        add_cost
    }

    pub fn sub_position(&mut self, timestamp: u64, current_price: f64, quantity: f64) -> f64 {
        if quantity <= self.quantity {
            self.quantity -= quantity;
            self.update_position_profit(current_price);
            if self.quantity == 0.0 {
                self.close_time = Some(timestamp);
            }
            current_price * quantity
        } else {
            warn!("Can't sell quantity more then held.");
            0.0
        }
    }

    pub fn update_position_profit(&mut self, current_price: f64) {
        // 更新仓位盈利信息
        let (open_price, quantity) = (self.open_price, self.quantity);
        // 持仓成本 = 持仓均价 * 持仓数量
        let held_cost = open_price * quantity;
        // 收益 = 最新持仓成本 - （现价 * 最新持仓数量）
        self.profit = (current_price - open_price) * quantity;
        // 收益率 = 收益 / 持仓成本
        self.profit_ratio = if held_cost != 0.0 {
            self.profit / held_cost * 100.0
        } else {
            0.0
        };
    }
}
pub trait Trading {
    // 下单
    fn order(&mut self, info: Order) -> Result<u64>;
    // 取消单
    fn cancel(&mut self, id: u64) -> bool;
    // 获取委托单状态
    fn delegate(&self, id: u64) -> Option<OrderState>;
    // 获得仓位信息
    fn position(&self, code: &str) -> Vec<&Position>;
}
