#[macro_export]
macro_rules! build_flow {
    // Complete version with proper-edge handling
    (
        nodes: [$(($name:expr, $node:expr)),* $(,)?],
        edges: [
            $($edge:tt),* $(,)?
        ]
    ) => {{
        let mut builder = $crate::GraphBuilder::new();
        // Add all nodes first
        $(
            builder = builder.add_node($name, $node);
        )*
        // Handle edges appropriately
        $(
            builder = $crate::build_flow!(@edge builder, $edge);
        )*
        builder.build()
    }};


    (@edge $builder:expr, ($from:expr, $to:expr)) => {
        $builder.add_edge($from, $to)
    };

    (@edge $builder:expr, ($from:expr, $to:expr, $condition:expr)) => {
        $builder.add_condition_edge($from, $to, $condition)
    };
}
