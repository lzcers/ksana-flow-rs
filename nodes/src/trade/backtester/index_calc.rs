use ndarray::Array;
use super::trading::Position;

pub fn calc_positions_value(positions: &[Position]) -> f64 {
    let mut asset_value = 0.0;
    for p in positions {
        if p.close_time.is_none() {
            asset_value += p.quantity * p.open_price + p.profit;
        }
    }
    asset_value
}

// 年化率
pub fn calc_profit_rate_year(init: f64, end: f64, days: f64) -> f64 {
    (end / init).powf(252.0 / days) - 1.0
}

// 夏普率计算, 无风险利率以 0.04 计算
pub fn calc_sharpe_rate(profit_rate_year: f64, profit_days: &Vec<f64>) -> f64 {
    let profit_std = Array::from_vec(profit_days.clone()).std(0.0);
    (profit_rate_year - 0.04) / (profit_std * (252.0_f64).sqrt())
}

pub fn calc_max_drawdown(values: &Vec<f64>) -> f64 {
    let mut max_drawdown = 0.0;
    if values.len() == 0 {
        return 0.0;
    }
    let mut peak = values[0];
    for value in values.iter().skip(1) {
        if *value > peak {
            peak = *value;
        } else {
            let drawdown = (peak - value) / peak;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
    }
    max_drawdown
}

// 胜率 = 盈利交易次数 / 总交易次数;
pub fn calc_win_rate(profits: Vec<f64>) -> (u32, u32, f64) {
    let mut win_nums = 0;
    let mut loss_nums = 0;
    for profit in profits {
        if profit > 0.0 {
            win_nums += 1;
        } else if profit < 0.0 {
            loss_nums += 1;
        }
    }
    return (
        win_nums,
        loss_nums,
        win_nums as f64 / (win_nums + loss_nums) as f64,
    );
}
