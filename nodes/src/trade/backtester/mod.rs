pub mod engine;
mod index_calc;
pub mod trading;

use async_trait::async_trait;
use flow::{Context, Input, Node, Output};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    async fn run(
        &mut self,
        _ctx: &Context,
        input: &Input,
    ) -> Result<Output, String> {
        let input: BacktesterInput = input
            .get_any_as()
            .ok_or_else(|| "Backtester expected BacktesterInput".to_string())?;

        let _ = self.order(input.order);
        self.update(&input.k);
        // self.print_backtest_result();
        let out = BacktesterOutput {
            balance: self.get_balance(),
            positions: self.get_positions().clone(),
            trading_record: self.get_trading_record().clone(),
        };
        Ok(serde_json::to_value(out).unwrap_or(Value::Null).into())
    }
}
