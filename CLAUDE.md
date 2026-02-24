# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Kśana Flow** (刹那流) is a Rust-based workflow engine for building LLM Agent applications. It provides a graph-based node execution framework where computation flows are modeled as directed graphs.

Key components:
- **flow**: Core workflow engine with graph execution, runner, and reactive streams
- **nodes**: Node implementations (LLM, text processing, MapReduce, trading, etc.)
- **agent**: Standalone LLM Agent crate with Provider/Model/Tool abstraction
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

`server/src/registry.rs` maintains a registry of node creators. New nodes must be registered here with their metadata (name, config schema, input/output types) and a factory function that creates the node from JSON config.

### Agent Module Architecture

The `agent` crate provides a standalone LLM Agent implementation:

1. **Core Types** (`core.rs`): `Message` enum (System/User/Assistant/Tool), `Usage`, `MessageRole`
2. **Models** (`models/`):
   - `ChatCapability`: Trait for chat models (non-streaming and streaming)
   - `GenImgCapability`: Trait for image generation models
   - `ChatModel`: Multi-provider chat model (DeepSeek, OpenRouter) with dynamic model mapping
   - `GenImgModel`: Image generation model
3. **Providers** (`providers/`): DeepSeek and OpenRouter API implementations
4. **Agents** (`agents/`):
   - `Agent`: Orchestrates model chat + tool execution loop with max iterations
   - `ToolExecutor`: Trait for executing tool calls
   - `ToolDef`: Tool definition schema for model consumption
   - `ToolRegistry`: Registry of available tools
   - `WebAgent`: Specialized agent with browser automation via Playwright CLI

## Development Commands

### Build
```bash
# Build entire workspace
cargo build

# Build release version
cargo build --release

# Build specific package
cargo build -p flow
cargo build -p nodes
cargo build -p agent
cargo build -p server
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
cargo test -p agent

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
        config: serde_json::json!({...}),
        inputs: vec![InputType::String],
        outputs: vec![InputType::String],
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

### Using the Agent Module

The `agent` crate provides a standalone LLM Agent implementation with streaming support.

#### Basic Usage (Non-streaming)

```rust
use agent::{
    agents::{Agent, Tool, ToolDef, GenericToolExecutor, ToolExecutorError},
    core::Message,
    models::ChatModel,
    providers::DeepSeekProvider,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

// Define a custom tool
struct SearchTool {
    def: ToolDef,
}

impl SearchTool {
    fn new() -> Self {
        Self {
            def: ToolDef {
                name: "search".to_string(),
                description: "Search the web".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"]
                }),
            },
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> &ToolDef {
        &self.def
    }

    async fn execute(&self, arguments: Value) -> Result<Value, ToolExecutorError> {
        Ok(serde_json::json!({"result": "search result"}))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create chat model with provider
    let mut model = ChatModel::new();
    let provider = Arc::new(DeepSeekProvider::from_env()?);

    model.add_model_provider("deepseek-chat", provider);
    model.set_active_model("deepseek-chat")?;

    // Create tool executor
    let mut tool_executor = GenericToolExecutor::new();
    tool_executor.register(SearchTool::new());

    // Create agent
    let agent = Agent::new(model, tool_executor)
        .with_max_iterations(10);

    // Run agent (non-streaming, blocks until complete)
    let messages = vec![Message::user("Hello!")];
    let result = agent.run(messages).await?;

    Ok(())
}
```

#### Streaming Usage (Recommended)

```rust
use agent::{
    agents::{Agent, AgentEvent, GenericToolExecutor},
    core::Message,
    models::ChatModel,
    providers::DeepSeekProvider,
};
use futures::StreamExt;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup model and executor as above...
    let mut model = ChatModel::new();
    let provider = Arc::new(DeepSeekProvider::from_env()?);
    model.add_model_provider("deepseek-chat", provider);
    model.set_active_model("deepseek-chat")?;

    let tool_executor = GenericToolExecutor::new();
    let agent = Agent::new(model, tool_executor);

    let messages = vec![Message::user("What is the weather today?")];

    // Run agent with streaming events
    let mut stream = agent.run_stream(messages).await?;

    while let Some(event) = stream.next().await {
        match event? {
            AgentEvent::AssistantMessage(msg) => {
                println!("Assistant replied: {:?}", msg);
            }
            AgentEvent::ToolCalls(calls) => {
                println!("Tool calls detected: {} calls", calls.len());
            }
            AgentEvent::ToolResult { call_id, success, output } => {
                println!("Tool {} completed: success={}", call_id, success);
            }
            AgentEvent::Iteration { iteration, message_count } => {
                println!("Iteration {} complete, {} messages", iteration, message_count);
            }
            AgentEvent::Complete(messages) => {
                println!("Agent completed with {} messages", messages.len());
                break;
            }
        }
    }

    Ok(())
}
```

#### AgentEvent Types

| Event | Description |
|-------|-------------|
| `AssistantMessage(Message)` | Model generated a response |
| `ToolCalls(Vec<ToolCall>)` | Model requested tool execution |
| `ToolResult { call_id, success, output }` | Single tool completed |
| `Iteration { iteration, message_count }` | One chat+tool cycle completed |
| `Complete(Vec<Message>)` | Agent finished, contains full conversation |

#### Key Design Changes

1. **`run_stream` is the primary method** - `run` is now a wrapper around `run_stream`
2. **Partial tool failures are handled gracefully** - One tool failing doesn't stop others
3. **Arc<Mutex<>> internal design** - Agent can be used with `&self`, no `&mut` needed
4. **Real-time observability** - See each step of the agent loop as it happens
