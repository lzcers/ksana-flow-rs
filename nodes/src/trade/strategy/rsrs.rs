use crate::trade::{
    backtester::trading::Order,
    k::{High, K, Low},
};
use async_trait::async_trait;
use flow::Node;
use ndarray::Array;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSRSNode {
    period: usize,
    std_period: usize,
    low_list: Vec<f64>,
    high_list: Vec<f64>,
    beta_list: Vec<f64>,
}

impl RSRSNode {
    pub fn new(period: usize, std_period: usize) -> Self {
        Self {
            period,
            std_period,
            low_list: vec![],
            high_list: vec![],
            beta_list: vec![],
        }
    }

    fn calculate_beta(&self, high: &[f64], low: &[f64]) -> f64 {
        let low_list = Array::from_vec(low.to_vec());
        let high_list = Array::from_vec(high.to_vec());

        let low_mean = low_list.mean().unwrap_or(0.0);
        let high_mean = high_list.mean().unwrap_or(0.0);

        let low_loss = &low_list - low_mean;
        let high_loss = &high_list - high_mean;

        let denominator = low_loss.dot(&low_loss);
        if denominator == 0.0 {
            0.0
        } else {
            low_loss.dot(&high_loss) / denominator
        }
    }

    fn zscore(&self) -> f64 {
        if self.beta_list.len() < self.std_period {
            return 0.0;
        }

        let beta_array = Array::from_vec(self.beta_list.clone());
        let mean = beta_array.mean().unwrap_or(0.0);
        let std = beta_array.std(0.0);

        if std == 0.0 {
            0.0
        } else {
            (self.beta_list.last().copied().unwrap_or(0.0) - mean) / std
        }
    }
}

#[async_trait]
impl Node for RSRSNode {
    type In = K;
    type Out = Option<Order>;

    async fn run(&mut self, _ctx: &flow::Context, input: Self::In) -> Self::Out {
        self.low_list.push(input.low());
        self.high_list.push(input.high());

        if self.low_list.len() > self.period {
            self.low_list.remove(0);
            self.high_list.remove(0);
        }

        if self.low_list.len() == self.period {
            let beta = self.calculate_beta(&self.high_list, &self.low_list);
            self.beta_list.push(beta);

            if self.beta_list.len() > self.std_period {
                self.beta_list.remove(0);
            }

            let zscore = self.zscore();

            if zscore > 0.7 {
                Some(Order::BUY)
            } else if zscore < -0.7 {
                Some(Order::SELL)
            } else {
                None
            }
        } else {
            None
        }
    }
}
