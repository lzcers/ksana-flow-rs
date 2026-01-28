pub mod engine;
mod index_calc;
pub mod trading;

use async_trait::async_trait;
use flow::{Node, NodeInputs, OutputPayload};
use serde::{Deserialize, Serialize};

use crate::trade::k::K;

use self::{
    engine::{Backtester, Record},
    trading::{Order, Position, Trading},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktesterInput {
    pub k: K,
    pub order: Order,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktesterOutput {
    pub balance: f64,
    pub positions: Vec<Position>,
    pub trading_record: Vec<Record>,
}

#[async_trait]
impl Node for Backtester {
    async fn run(&mut self, _ctx: &flow::Context, inputs: NodeInputs) -> OutputPayload {
        let input = inputs
            .get_any()
            .and_then(|p| p.as_any())
            .and_then(|a| a.downcast_ref::<BacktesterInput>())
            .cloned()
            .expect("Backtester expected BacktesterInput");

        let _ = self.order(input.order);
        self.update(&input.k);
        // self.print_backtest_result();
        let out = BacktesterOutput {
            balance: self.get_balance(),
            positions: self.get_positions().clone(),
            trading_record: self.get_trading_record().clone(),
        };
        OutputPayload::cloned(out)
    }
}
