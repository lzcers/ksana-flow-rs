# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Kśana Flow** (刹那流) is a Rust-based workflow engine for building LLM Agent applications. It provides a graph-based node execution framework where computation flows are modeled as directed graphs.

Key components:
- **flow**: Core workflow engine with graph execution, runner, and reactive streams
- **nodes**: Node implementations (LLM, text processing, MapReduce, trading, etc.)
- **server**: Axum-based HTTP API and WebSocket server
- **web**: React/TypeScript frontend with visual workflow editor

## Architecture

### Core Execution Model

The execution model consists of:

1. **Graph**: A directed graph of nodes and edges defining the workflow structure
2. **Node**: The basic execution unit, implementing the `Node` trait with `run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String>`
3. **Context**: Thread-safe data store (`DashMap<String, Value>`) for sharing state between nodes
4. **Runner**: The graph executor that schedules and runs nodes based on their trigger strategy

### Trigger Strategies

Nodes can specify when they execute:
- `AllUpstreamReady` (default): Execute when all upstream nodes complete
- `AnyUpstreamAvailable`: Execute when any upstream node produces output

### Subgraph & MapReduce

Recent additions include:

1. **SubgraphExecutor**: Executes a graph within a graph, with isolated or inherited context. Used for encapsulating reusable workflows.

2. **MapNode**: Parallel processing node that distributes input array items to child nodes. Configurable concurrency via `max_parallel` semaphore.

3. **SubgraphMapNode**: Combines Map with Subgraph - each item is processed by a subgraph instance.

### Node Registry

`server/src/registry.rs` maintains a registry of node creators. New nodes must be registered here with their metadata (name, description, category, config schema) and a factory function that creates the node from JSON config.

## Development Commands

### Build
```bash
# Build entire workspace
cargo build

# Build release version
cargo build --release

# Build specific package
cargo build -p flow
cargo build -p server
cargo build -p nodes
```

### Run
```bash
# Run the server (backend + API)
cargo run -p server

# Development (PowerShell) - starts both backend and frontend
./start_dev.ps1

# On Windows with bash
cargo run -p server &
cd web && npm run dev
```

### Test
```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test -p flow
cargo test -p nodes

# Run specific test by name
cargo test test_complex_graph_connections

# Run with output visible
cargo test -- --nocapture
```

### Lint
```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Fix automatically fixable issues
cargo clippy --fix
cargo fmt
```

### Frontend (web/)
```bash
cd web

# Install dependencies
npm install
# or
pnpm install

# Start dev server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

### Database
The server uses SQLite (`ksana.db` in project root). Schema is initialized automatically on startup.

### Environment Variables
Create `.env` file in project root:
```
DEEPSEEK_API_KEY=sk-...
OPENROUTER_API_KEY=sk-or-v1-...
```

## Key Patterns

### Creating a New Node

1. Define config struct with `Deserialize`:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MyNodeConfig {
    pub param: String,
}
```

2. Implement the `Node` trait:
```rust
#[async_trait]
impl Node for MyNode {
    async fn run(&mut self, ctx: &Context, input: &Input) -> Result<Output, String> {
        // Implementation
        Ok(Output::new(value))
    }
}
```

3. Register in `server/src/registry.rs`:
```rust
registry.register(
    NodeMetadata {
        name: "MyNode".to_string(),
        description: "...".to_string(),
        category: "...".to_string(),
        config: serde_json::json!({...}),
        inputs: vec![...],
        outputs: vec![...],
    },
    |config| {
        let cfg: MyNodeConfig = serde_json::from_value(config)?;
        Ok(Arc::new(RwLock::new(MyNode::new(cfg))) as Arc<RwLock<dyn AnyNode>>)
    },
);
```

### Testing with the Runner

```rust
#[tokio::test]
async fn test_my_workflow() {
    let graph = GraphBuilder::new()
        .add_node("input", InputNode::new("hello"))
        .add_node("process", ProcessNode::new())
        .add_edge("input", "process")
        .build();

    let (runner, handle) = Runner::new(graph, None);
    let result = runner.run(Input::new(...)).await;

    assert!(result.is_ok());
}
```
