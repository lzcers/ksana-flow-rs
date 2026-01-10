use anyhow::Result;
use chrono::{Duration, Local};
use flow::{Context, Runner, build_flow};
use nodes::{
    TimerNode,
    trade::{Backtester, K, SourceNode, VOLMFINode},
};
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let flow = build_flow! {
        // start: ("timer", TimerNode::new("*/10 * * * * *")?),
        nodes: [
            ("source", SourceNode::new("510300.SH",Local::now() - Duration::days(100), None )?),
            ("vol_mfi_strategy", VOLMFINode::new(8, 8)),
            ("backtesterNode", Backtester::new(100000.0, 0.0002354)),
        ],
        edges: [
            ("source", "vol_mfi_strategy"),
            ("source", "source",  |_ctx: &Context, output: &Option<K>| output.is_some()),
            ("vol_mfi_strategy", "backtesterNode"),
        ]
    };

    let rtx = tokio::runtime::Runtime::new()?;
    let mut runner = Runner::new(flow).set_start_node("source", &());
    let err = rtx.block_on(async move { runner.run().await });
    println!("{:?}", err);
    Ok(())
}
