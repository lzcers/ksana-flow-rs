use crate::trade::{backtester::trading::Order, k::K};
use async_trait::async_trait;
use flow::Node;
use serde::{Deserialize, Serialize};
use ta::Next;
use ta::indicators::SimpleMovingAverage as Sma;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SMANode {
    p1: usize,
    p2: usize,
    #[serde(skip)]
    sma1: Option<Sma>,
    #[serde(skip)]
    sma2: Option<Sma>,
    last_e1: f64,
    last_e2: f64,
}

impl SMANode {
    pub fn new(p1: usize, p2: usize) -> Self {
        Self {
            p1,
            p2,
            sma1: Some(Sma::new(p1).unwrap()),
            sma2: Some(Sma::new(p2).unwrap()),
            last_e1: 0.0,
            last_e2: 0.0,
        }
    }
}

#[async_trait]
impl Node for SMANode {
    type In = K;
    type Out = Option<Order>;

    async fn run(&mut self, _ctx: &flow::Context, input: Self::In) -> Self::Out {
        if self.sma1.is_none() {
            self.sma1 = Some(Sma::new(self.p1).unwrap());
        }
        if self.sma2.is_none() {
            self.sma2 = Some(Sma::new(self.p2).unwrap());
        }

        let sma1 = self.sma1.as_mut().unwrap();
        let sma2 = self.sma2.as_mut().unwrap();

        let e1 = sma1.next(input.close);
        let e2 = sma2.next(input.close);

        let signal = if self.last_e1 <= self.last_e2 && e1 > e2 {
            Some(Order::BUY)
        } else if self.last_e1 >= self.last_e2 && e1 < e2 {
            Some(Order::SELL)
        } else {
            None
        };

        self.last_e1 = e1;
        self.last_e2 = e2;

        signal
    }
}
