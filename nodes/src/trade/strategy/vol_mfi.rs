use crate::trade::{
    backtester::{BacktesterInput, trading::Order},
    k::K,
};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use flow::{Node, NodeInputs};
use ta::Next;
use ta::indicators::{ExponentialMovingAverage, MoneyFlowIndex};

pub struct VOLMFINode {
    vol_ema_index: ExponentialMovingAverage,
    mfi_index: MoneyFlowIndex,
    prev_mfi: Option<f64>,
    prev_vol_ema: Option<f64>,
}

impl VOLMFINode {
    pub fn new(ema_period: usize, mfi_period: usize) -> Self {
        Self {
            vol_ema_index: ExponentialMovingAverage::new(ema_period)
                .expect("vol_ema_index create failed"),
            mfi_index: MoneyFlowIndex::new(mfi_period).expect("mfi_index  create failed"),
            prev_mfi: None,
            prev_vol_ema: None,
        }
    }
}

#[async_trait]
impl Node for VOLMFINode {
    type Out = BacktesterInput;

    async fn run(&mut self, _ctx: &flow::Context, inputs: NodeInputs) -> Self::Out {
        let input = inputs
            .get_any()
            .and_then(|any| any.as_ref().as_any().downcast_ref::<K>())
            .cloned()
            .expect("VOLMFINode expected K input");

        let k = input;
        let mfi = self.mfi_index.next(&k);
        let vol_ema = self.vol_ema_index.next(k.volume);

        let (k, d_vol_ema, d_rov, d_mfi) =
            if let (Some(prev_mfi), Some(prev_vol_ema)) = (self.prev_mfi, self.prev_vol_ema) {
                calc_strategy_index(Some((k.clone(), prev_mfi, prev_vol_ema)), (k, mfi, vol_ema))
            } else {
                calc_strategy_index(None, (k, mfi, vol_ema))
            };
        self.prev_mfi = Some(mfi);
        self.prev_vol_ema = Some(vol_ema);

        let signal = gen_trading_signal(&k, d_vol_ema, d_rov, d_mfi);

        BacktesterInput {
            k,
            order: signal.signal_type,
        }
    }
}

// d_vol = now.volume - p_vol_ema  计算当前成交量与过去交易量移动平均值的差
// d_rovc = d_vol / p_vol_ema 计算当前交易量的变化与过去交易量移动平均值的比例，即交易量的变化率
// d_mfi = now.mfi - p_mfi 计算当前 MFI 值与过去 MFI 值的差值

// (k, mfi, vol_ema)
type StrategyParams = (K, f64, f64);

fn calc_strategy_index(prev: Option<StrategyParams>, now: StrategyParams) -> (K, f64, f64, f64) {
    let Some((_p_k, p_mfi, p_vol_ema)) = prev else {
        return (now.0, 0.0, 0.0, 0.0);
    };
    let (k, mfi, _) = now;
    // 指标计算
    let d_vol_ema = k.volume - p_vol_ema;
    let d_rov = k.volume / p_vol_ema - 1.0;
    let d_mfi = mfi - p_mfi;
    (k, d_vol_ema, d_rov, d_mfi)
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TradingSignal {
    signal_type: Order,
    timestamp: NaiveDateTime,
    strategy_id: String,
}

// 我们的策略是判断 d_vol 是否大于 0，即交易量相比过去的平均值是否是增加的，即交易量绝对值是否增长
// 同时判断交易量的变化率是否大于 0，即交易量的变化的幅度
// 再辅助判断 MFI 值是否大于 0，即 MFI 值是否是增加的，资金流入是否增强
fn gen_trading_signal(k: &K, d_vol_ema: f64, d_rov: f64, d_mfi: f64) -> TradingSignal {
    let d_rov_threshold = 0.05;
    let timestamp = chrono::DateTime::from_timestamp_millis(k.timestamp as i64)
        .unwrap()
        .naive_utc();

    if d_vol_ema > 0.0 && d_rov > d_rov_threshold && d_mfi > 0.0 {
        TradingSignal {
            signal_type: Order::BUY,
            timestamp,
            strategy_id: "vol_mfi_confirmation".to_string(),
        }
    } else if d_vol_ema > 0.0 && d_rov > 0.0 && d_mfi < 0.0 {
        TradingSignal {
            signal_type: Order::SELL,
            timestamp,
            strategy_id: "vol_mfi_confirmation".to_string(),
        }
    } else {
        TradingSignal {
            signal_type: Order::HOLD,
            timestamp,
            strategy_id: "vol_mfi_confirmation".to_string(),
        }
    }
}
