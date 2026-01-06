use super::index_calc::{
    calc_max_drawdown, calc_positions_value, calc_profit_rate_year, calc_sharpe_rate, calc_win_rate,
};
use super::k::K;
use super::trading::{Order, OrderInfo, OrderState, Position, Trading};
use anyhow::Result;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use tracing::warn;

pub fn timestamp_to_str(timestamp: u64) -> String {
    let datetime = DateTime::from_timestamp_millis(timestamp as i64).unwrap();
    datetime.date_naive().to_string()
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Record {
    order: OrderInfo,
    code: String,
    timestamp: u64,
    price: f64,
    quantity: f64,
    profit: f64,
    fee: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Value {
    timestamp: u64,
    amount: f64,
    profit_rate: f64,
}

pub struct Backtester {
    id_index: u64,
    order_list: Vec<OrderInfo>,
    init_money: f64,
    balance: f64,
    positions: Vec<Position>,
    fee_rate: f64,
    // 交易记录
    trading_record: Vec<Record>,
    // 资产变化记录
    value_record: Vec<Value>,
}

impl Backtester {
    pub fn new(init_money: f64, fee_rate: f64) -> Self {
        Self {
            id_index: 0,
            order_list: vec![],
            balance: init_money,
            init_money,
            positions: vec![],
            fee_rate: fee_rate,
            trading_record: vec![],
            value_record: vec![],
        }
    }

    pub fn get_balance(&self) -> f64 {
        self.balance
    }

    pub fn get_fee_rate(&self) -> f64 {
        self.fee_rate
    }

    pub fn get_trading_record(&self) -> &Vec<Record> {
        &self.trading_record
    }

    pub fn get_positions(&self) -> &Vec<Position> {
        &self.positions
    }

    pub fn trading(amount: f64, fee_rate: f64) -> f64 {
        amount + (amount * fee_rate)
    }

    // 胜率 = 盈利交易次数 / 总交易次数;
    // 最大回撤 = 现金+股票最大价值时 - 现金+股票最小价值 / 现金+股票最大价值时
    pub fn get_backtest_result(&self) {
        println!("");
        println!("");
        println!("--------------- Backtest result: ---------------");

        println!("\nTrading record: \n");
        for record in &self.trading_record {
            println!("{} {:?}", timestamp_to_str(record.timestamp), record);
        }

        println!("\nValue record: \n");
        for record in &self.value_record {
            println!("{} {:?}", timestamp_to_str(record.timestamp), record);
        }

        let current_blance = self.get_balance();
        let assets_value = calc_positions_value(&self.positions);

        // 年化率
        let year_profit_rate = calc_profit_rate_year(
            self.init_money,
            current_blance + assets_value,
            self.value_record.len() as f64,
        );

        // 胜率
        let (win_nums, loss_nums, win_rate) =
            calc_win_rate(self.trading_record.iter().map(|t| t.profit).collect());

        // 最大回撤

        let max_drawdown_rate =
            calc_max_drawdown(&self.value_record.iter().map(|r| r.amount).collect());

        // 夏普率计算, 无风险利率以 0.04 计算
        let profits = self
            .value_record
            .iter()
            .map(|v| v.profit_rate)
            .collect::<Vec<f64>>();
        let sharpe_rate = calc_sharpe_rate(year_profit_rate, &profits);

        println!(
            "\nblance: {} held funds: {}, total: {}",
            current_blance,
            assets_value,
            current_blance + assets_value,
        );

        println!(
            "\ndeal count: {} win: {}, loss: {}, win-rate: {}, max-drawdown: {}, year_profit_rate: {}, sharpe_rate: {}",
            win_nums + loss_nums,
            win_nums,
            loss_nums,
            win_rate * 100.0,
            max_drawdown_rate * 100.0,
            year_profit_rate * 100.0,
            sharpe_rate,
        );
    }

    fn add_record(
        record: &mut Vec<Record>,
        order_info: &OrderInfo,
        code: &str,
        timestamp: u64,
        price: f64,
        quantity: f64,
        profit: f64,
    ) {
        record.push(Record {
            order: order_info.clone(),
            code: code.to_owned(),
            timestamp,
            price,
            quantity,
            profit,
            fee: 0.0,
        });
    }

    /// 获取当前仓位
    fn get_position<'a>(positions: &'a mut [Position], code: &str) -> Option<&'a mut Position> {
        positions
            .iter_mut()
            .find(|p| p.code == code && p.close_time.is_none())
    }

    fn update_positions_profit(&mut self, current_price: f64) {
        self.positions.iter_mut().for_each(|p| {
            if p.close_time.is_none() {
                p.update_position_profit(current_price);
            }
        });
    }

    // 遍历所有委托单
    // 1. 判断当前 K 线下是否可以成交
    // 2. 成交则按照交易价格更新仓位信息
    // 撮合机制
    // 回测频率选择“每日”时，系统根据委托价格与当日价格最低及最高点进行比较来判定是否成交（即“K棒拟合”算法）。
    // 以买入委托为例，如果发出限价委托，当委托买价高于当日最低价时，则判定发生成交。
    // 成交价格根据委托价格是否高于K线均价分为两种情况：
    // 当委托价格小于K线均价时，成交价即为委托价。当委托价格高于K线均价时，成交价判定为（委托价+K线均价）/2.
    // 成交数量根据当天成交量的三角分布模型判定。
    // 如果发出市价买入委托，对成交价的判定为（当日K线最高价+当日K线均价）/2。成交数量依然根据当天成交量的三角分布模型判定。
    // 在日级别回测时，系统并不进行开盘集合竞价，每个交易日只进行一次成交判定，未成交的委托将被自动撤单（相当于FAK）。
    // 在“每日”级别回测时，系统只会在每日的15：00发送一次行情，如果有成交，系统则以这个时间15：00来显示成交时间。
    pub fn update(&mut self, k: &K) {
        // 以当日均价作为现价
        // 假设每次都是全仓买入，能买多少手
        let cur_price = (k.high + k.low) / 2.0;
        // 头寸大小
        let available_balance = self.balance * 0.10;

        // 更新所有仓位的收益信息
        self.update_positions_profit(cur_price);

        // 处理所有委托单
        self.order_list.retain(|order_info| {
            let position = Self::get_position(&mut self.positions, &k.code);
            match order_info.order {
                Order::BUY => {
                    let hand_num = (available_balance / (cur_price * 100.0)).floor();
                    // 仓位
                    let quantity = hand_num * 100.0;
                    if quantity > 0.0 {
                        if let Some(current_position) = position {
                            // 加仓
                            let fee = current_position.add_position(cur_price, quantity);
                            self.balance -= Self::trading(fee, self.fee_rate);
                            // 写入记录
                            Self::add_record(
                                &mut self.trading_record,
                                order_info,
                                &k.code,
                                k.timestamp,
                                cur_price,
                                quantity,
                                current_position.profit,
                            );
                        } else {
                            // 开新仓
                            let position =
                                Position::open_position(&k.code, k.timestamp, cur_price, quantity);
                            self.positions.push(position);
                            self.balance -= Self::trading(cur_price * quantity, self.fee_rate);

                            // 写入记录
                            Self::add_record(
                                &mut self.trading_record,
                                order_info,
                                &k.code,
                                k.timestamp,
                                cur_price,
                                quantity,
                                0.0,
                            );
                        }
                    } else {
                        warn!("Blance is unsufficent, unable to open position");
                    }
                }
                Order::SELL => {
                    // 假设都以市价卖出，每次都是全仓卖出
                    let sell_price = cur_price;
                    // 每次卖掉单位数
                    if let Some(position) = position {
                        if position.quantity > 0.0 {
                            let sell_quantity = (position.quantity * 1.0 / 100.0).floor() * 100.0;
                            let profit = position.profit;
                            let income =
                                position.sub_position(k.timestamp, sell_price, sell_quantity);
                            self.balance += Self::trading(income, self.fee_rate);
                            // 写入记录
                            Self::add_record(
                                &mut self.trading_record,
                                order_info,
                                &k.code,
                                k.timestamp,
                                cur_price,
                                sell_quantity,
                                profit,
                            );
                        }
                    }
                }
            }
            return false;
        });

        let assets_value = calc_positions_value(&self.positions);
        let amount = assets_value + self.balance;
        let prev_value = self.value_record.last();
        let profit_rate = if let Some(v) = prev_value {
            // 今日收益率 = 今日资产总价值 - 昨日资产总价值
            (amount - v.amount) / v.amount
        } else {
            (amount - self.init_money) / self.init_money
        };

        self.value_record.push(Value {
            timestamp: k.timestamp,
            amount,
            profit_rate,
        })
    }
}

impl Trading for Backtester {
    /// 创建委托单
    fn order(&mut self, info: Order) -> Result<u64> {
        let new_id: u64 = self.id_index + 1;
        let order_info = OrderInfo {
            id: new_id,
            order: info,
        };
        self.order_list.push(order_info);
        self.id_index += 1;
        Ok(new_id)
    }

    fn cancel(&mut self, id: u64) -> bool {
        self.order_list.retain(|order| order.id != id);
        true
    }

    fn delegate(&self, _id: u64) -> Option<OrderState> {
        todo!()
    }

    fn position(&self, code: &str) -> Vec<&Position> {
        self.positions
            .iter()
            .filter(|p| p.code == code)
            .collect::<Vec<&Position>>()
    }
}

impl Default for Backtester {
    fn default() -> Self {
        Self::new(100000.0, 0.0003)
    }
}
