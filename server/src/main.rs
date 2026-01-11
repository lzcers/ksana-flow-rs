use anyhow::Result;
use chrono::{Duration, Local};
use flow::{Runner, build_flow};
use nodes::trade::{Backtester, ReactiveSourceNode, VOLMFINode};
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let flow = build_flow! {
        nodes: [
            ("source", ReactiveSourceNode::new("510300.SH", Local::now() - Duration::days(300), None)?),
            ("vol_mfi_strategy", VOLMFINode::new(8, 8)),
            ("backtesterNode", Backtester::new(500000.0, 0.0002354)),
        ],
        edges: [
            ("source", "vol_mfi_strategy"),
            ("vol_mfi_strategy", "backtesterNode"),
        ]
    };

    let rtx = tokio::runtime::Runtime::new()?;
    let mut runner = Runner::new(flow).set_start_node("source", &());
    let err = rtx.block_on(async move { runner.run().await });
    println!("{:?}", err);
    Ok(())
}
