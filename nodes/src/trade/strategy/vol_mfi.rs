use crate::trade::{backtester::trading::Order, k::K};
use async_trait::async_trait;
use flow::Node;
use serde::{Deserialize, Serialize};
use ta::DataItem;
use ta::Next;
use ta::indicators::{ExponentialMovingAverage as Ema, MoneyFlowIndex as Mfi};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VOLMFINode {
    ema_period: usize,
    mfi_period: usize,
    #[serde(skip)]
    vol_ema: Option<Ema>,
    #[serde(skip)]
    mfi: Option<Mfi>,
    prev_mfi: f64,
    prev_vol_ema: f64,
}

impl VOLMFINode {
    pub fn new(ema_period: usize, mfi_period: usize) -> Self {
        Self {
            ema_period,
            mfi_period,
            vol_ema: Some(Ema::new(ema_period).unwrap()),
            mfi: Some(Mfi::new(mfi_period).unwrap()),
            prev_mfi: 0.0,
            prev_vol_ema: 0.0,
        }
    }
}

#[async_trait]
impl Node for VOLMFINode {
    type In = K;
    type Out = Option<Order>;

    async fn run(&mut self, _ctx: &flow::Context, input: Self::In) -> Self::Out {
        if self.vol_ema.is_none() {
            self.vol_ema = Some(Ema::new(self.ema_period).unwrap());
        }
        if self.mfi.is_none() {
            self.mfi = Some(Mfi::new(self.mfi_period).unwrap());
        }

        let vol_ema_ind = self.vol_ema.as_mut().unwrap();
        let mfi_ind = self.mfi.as_mut().unwrap();

        // Prepare DataItem for ta crate
        let data_item = DataItem::builder()
            .open(input.open)
            .high(input.high)
            .low(input.low)
            .close(input.close)
            .volume(input.volume)
            .build()
            .unwrap();

        let current_mfi = mfi_ind.next(&data_item);
        let current_vol_ema = vol_ema_ind.next(input.volume);

        let d_vol_ema = input.volume - self.prev_vol_ema;
        let d_rov = if self.prev_vol_ema != 0.0 {
            input.volume / self.prev_vol_ema - 1.0
        } else {
            0.0
        };
        let d_mfi = current_mfi - self.prev_mfi;

        let d_rov_threshold = 0.05;
        let signal = if d_vol_ema > 0.0 && d_rov > d_rov_threshold && d_mfi > 0.0 {
            Some(Order::BUY)
        } else if d_vol_ema > 0.0 && d_rov > 0.0 && d_mfi < 0.0 {
            Some(Order::SELL)
        } else {
            None
        };

        self.prev_mfi = current_mfi;
        self.prev_vol_ema = current_vol_ema;

        signal
    }
}
