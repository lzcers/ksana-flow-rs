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

The `agent` crate provides a standalone LLM Agent implementation using an **Actor-based design** with streaming support, state persistence, and layered context management.

#### Directory Structure

```
agent/src/
├── lib.rs           # Module exports
├── core.rs          # Core types: Message, Usage, MessageRole
├── models/
│   ├── mod.rs       # ChatCapability, GenImgCapability traits
│   ├── chat_model.rs    # ChatModel implementation
│   └── gen_img_model.rs # Image generation model
├── providers/
│   ├── mod.rs           # Provider trait, re-exports
│   ├── types.rs         # Request, Response, StreamResponse, Usage, ProviderError
│   ├── utils.rs         # parse_api_error, parse_sse_line
│   ├── openai_compatible.rs  # OpenAI-compatible base provider
│   ├── deepseek.rs      # DeepSeek API provider
│   └── openrouter.rs    # OpenRouter API provider
└── agents/
    ├── mod.rs               # Module exports
    ├── agent_actor/
    │   ├── mod.rs           # AgentActor: thin facade owning state, chat, tool_executor
    │   ├── builder.rs       # AgentActorBuilder
    │   ├── lifecycle.rs     # StepLifeCycle + LifeCycle phases + hooks pipeline
    │   ├── loop_control.rs  # run_loop + pause/resume/cancel state machine
    │   └── types.rs         # Errors, events, commands, handle, StepResult
    ├── agent_state.rs       # AgentState, JobState - state management
    ├── call_model.rs        # Pure functions: call_model, call_tool, call_tools
    ├── context.rs           # Context, Layer, LayerKind - layered context
    ├── hooks/
    │   ├── mod.rs           # LifeCycleHook trait + HookName type alias
    │   ├── execution_policy.rs  # Max-iteration guard hook
    │   ├── metrics.rs       # Timing metrics hook
    │   └── token_statistics.rs  # Token statistics hook (WIP)
    └── tools/
        ├── mod.rs           # ToolDef, ToolCall, ToolExecutor trait
        ├── registry.rs      # ToolRegistry, GenericToolExecutor
        └── playwright_cli.rs # Browser automation tool
```

#### Core Types (`core.rs`)

- **Message**: Tagged union with System/User/Assistant/Tool variants
  - Assistant messages support `reasoning_content` (DeepSeek reasoner mode)
  - Assistant messages can contain `tool_calls`
- **Usage**: Token usage statistics

#### Models (`models/`)

- **ChatCapability**: Trait for chat models
  - `chat()`: Non-streaming chat
  - `chat_stream()`: Streaming chat returning `BoxStream<ChatChunk>`
- **ChatChunk**: Stream chunk with `content`, `reasoning_content`, `tool_calls`
- **ChatModel**: Multi-provider chat model
  - `add_model_provider(name, provider)`: Register a model with its provider
  - `set_active_model(name)`: Set the active model
  - Supports DeepSeek (chat/reasoner) and OpenRouter models

#### Providers (`providers/`)

- **Provider**: Trait for API providers
  - `chat()`: Non-streaming request
  - `chat_stream()`: Streaming request returning `BoxStream<StreamResponse>`
  - `name()`: Provider name for logging
- **OpenAICompatibleProvider**: Base implementation for OpenAI-compatible APIs
- **DeepSeekProvider**: DeepSeek API implementation (chat/reasoner models)
- **OpenRouterProvider**: OpenRouter API implementation
- **Request**: Request parameters with builder pattern
  - `with_stream()`, `with_temperature()`, `with_max_tokens()`, `with_tools()`
- **Response/StreamResponse**: OpenAI-compatible response types
- **Usage**: Token usage statistics (supports both OpenAI and DeepSeek formats)
- **ProviderError**: Error types for API operations

#### Agent Actor (`agents/agent_actor/`)

`AgentActor` is a thin facade that owns state, chat model, and tool executor:

- `mod.rs`: `AgentActor` struct with `state`, `chat`, `tool_executor` fields
- `builder.rs`: `AgentActorBuilder` for configuration
- `lifecycle.rs`: `StepLifeCycle` executes a single step through the hook pipeline
- `loop_control.rs`: `run_loop()` manages pause/resume/cancel state machine
- `types.rs`: `AgentError`, `AgentActorEvent`, `AgentActorCommand`, `AgentActorHandle`, `StepResult`

The hook system is the primary extension mechanism. `StepLifeCycle::new()` creates a default hook chain:

1. `ExecutionPolicyHook`: Max-iteration guard
2. `MetricsHook`: Timing metrics (model call, tool call durations)
3. `TokenStatisticsHook`: Token statistics (WIP)

**LifeCycle Phases:**

- `BeforeStep`: Step-level control (max iteration check, budget validation, checkpoint restore)
- `BeforeCallModel`: Pre-model call orchestration (prompt injection, context compression, tool filtering)
- `OnModelEvent`: Stream event handling (UI output, token counting, reasoning collection)
- `AfterCallModel`: Model response processing (result validation, retry/ fallback decision)
- `BeforeCallTools`: Pre-tool execution governance (permission check, parameter fix, timeout config)
- `AfterCallTools`: Tool result processing (result normalization, error mapping, caching)
- `AfterStep`: Step finalization (context persistence, iteration increment, state transition)

Basic usage:

```rust
// Create with builder
let actor = AgentActorBuilder::new(chat_model, tool_executor)
    .context(context)
    .max_iterations(10)
    .build();

// Option 1: Run loop (automatic iteration)
let handle = actor.run_loop();
// handle.pause().await;
// handle.resume().await;
// handle.cancel().await;
let events = handle.wait().await;

// Option 2: Manual step control
let result = actor.run_step(Some(event_tx)).await;
```

**AgentActorEvent Types:**

| Event | Description |
|-------|-------------|
| `ContentChunk(String)` | LLM text chunk |
| `ReasoningChunk(String)` | DeepSeek reasoner content chunk |
| `StepCompleted { content, reasoning_content, tool_calls }` | LLM response complete (before commit) |
| `StepFinalized { result }` | Step result committed to state |
| `ToolCalls(Vec<ToolCall>)` | Model requested tools |
| `ToolResult { call_id, success, output }` | Single tool completed |
| `Iteration { iteration, message_count }` | One cycle complete |
| `HookEvent { hook, kind, payload }` | Custom event from hooks |
| `MaxIterations { iteration }` | Reached max iterations |
| `Completed` | Agent finished |
| `Cancelled` | User cancelled |
| `Error(AgentError)` | Error occurred |

**StepResult Types:**

- `Continue { content, reasoning_content, tools_call, tools_result }`: Has tool calls, continue iteration
- `Done { content, reasoning_content }`: No tool calls, finished
- `Error(AgentError)`: Execution error

**LifeCycleHook Trait:**

```rust
#[async_trait::async_trait]
pub trait LifeCycleHook: Send + Sync {
    fn name(&self) -> HookName;
    fn priority(&self) -> i32 { 0 }
    fn on(&self, stage: &LifeCycle) -> bool;
    async fn handle(&mut self, ctx: &mut LifeCycleContext) -> LifeCycleFlow;
}
```

**LifeCycleContext Fields:**

- `stage`: Current `LifeCycle` phase
- `state`: `AgentState` (read/write)
- `frame`: `StepFrame` with `model_output` and `tools_result`
- `model_event`: Optional `CallModelEvent` (set during `OnModelEvent`)

#### State Management (`agents/agent_state.rs`)

- **AgentState**: Persistable agent state with Context integrated
  - `job_id`, `user_id`, `conversation_id`: Identifiers
  - `title`, `description`, `category`: Task metadata
  - `state`: Current `JobState`
  - `budget`, `actual_cost`: Resource tracking
  - `context`: Layered context (integrated, not separate)
  - `iteration`, `max_iterations`: Execution control

- **JobState**: Execution state machine
  - `Pending` → `Running` → `Completed`/`Failed`/`Cancelled`
  - Supports `Paused`, `WaitingInput` for interactive scenarios

**Note**: Context is now a field of AgentState (`state.context`), not a separate entity passed around. This simplifies state persistence and enables better encapsulation.

#### Layered Context (`agents/context.rs`)

Context is a layered data container supporting multiple data types:

```rust
let context = Context::new()
    .layer(Layer::new("system", LayerKind::System, json!("You are helpful."))
        .with_priority(100))
    .layer(Layer::new("soul", LayerKind::Soul, json!({
        "name": "Kśana",
        "role": "AI Assistant",
        "guidelines": ["Be helpful", "Be concise"]
    })))
    .layer(Layer::new("conversation", LayerKind::Conversation, json!([])));

// Convert to messages for LLM
let messages = context.to_messages();

// Add message to conversation
context.add_message(Message::user("Hello"));
```

**LayerKind Types:**
- `System`: System instructions
- `Soul`: Personality/character definition
- `User`: User profile
- `Memory`: Long-term memory
- `Conversation`: Chat history
- `Tools`: Tool definitions
- `Custom(String)`: Custom layers

#### Tool System (`agents/tools/`)

- **ToolDef**: Tool definition for model consumption
- **ToolCall**: Model's tool call request (OpenAI-compatible)
- **ToolResult**: Tool execution result
- **ToolExecutor**: Trait for executing tools
- **GenericToolExecutor**: Registry-based executor

```rust
// Define a tool
struct SearchTool { def: ToolDef }

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> &ToolDef { &self.def }
    async fn execute(&self, args: Value) -> Result<Value, ToolExecutorError> {
        Ok(json!({"result": "search result"}))
    }
}

// Register tools
let mut executor = GenericToolExecutor::new();
executor.register(SearchTool::new());
```

#### Execution Utilities (`agents/call_model.rs`)

Pure functions for execution:

- `call_model(model, messages, tools_def) -> Stream<CallModelEvent>`: Stream LLM response
  - **Note**: Parameter order is `(model, messages, tools_def)` - model first
- `call_tool(executor, call) -> CallToolResult`: Execute single tool
- `call_tools(executor, calls) -> Stream<CallToolResult>`: Execute tools in parallel

**CallModelEvent Types:**
- `TextChunk(String)`: LLM text chunk
- `ReasoningChunk(String)`: DeepSeek reasoner content chunk
- `Completed { content, reasoning_content, tools_call }`: LLM response complete
- `Error(String)`: Error occurred

**CallToolResult Fields:**
- `call_id`: Tool call ID
- `tool_name`: Tool name
- `success`: Whether execution succeeded
- `output`: JSON string output

#### Key Design Principles

1. **Separation of concerns**: Actor handles control plane, `call_model.rs` handles execution
2. **Streaming first**: `chat_stream` is primary, `chat` wraps it
3. **Layered context**: Context supports multiple data types with priorities
4. **State persistence**: `AgentState` is serializable (Context integrated inside)
5. **Control interface**: Pause/Continue/Cancel via `AgentActorHandle`
6. **Parallel tool execution**: Multiple tools execute concurrently
7. **Hook-driven lifecycle**: `StepLifeCycle` orchestrates execution through `LifeCycleHook` pipeline
8. **LifeCycleFlow control**: Hooks return `ControlFlow` to break or continue execution

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

The `agent` crate provides an Actor-based LLM Agent with streaming support and state management.

**Key Architecture Points:**
- Context is integrated into AgentState (accessed via `state.context`)
- `StepLifeCycle` orchestrates step execution through `LifeCycleHook` pipeline
- `agent_actor/` module: `mod.rs` (facade), `lifecycle.rs` (step execution), `loop_control.rs` (loop control), `builder.rs` (construction), `types.rs` (types)
- `call_model` parameter order: `(model, messages, tools_def)` - model first
- Default hooks: `ExecutionPolicyHook`, `MetricsHook`, `TokenStatisticsHook`

#### Basic Usage with AgentActor

```rust
use agent::{
    agents::{
        AgentActor, AgentActorBuilder, AgentActorEvent,
        Context, Layer, LayerKind,
        GenericToolExecutor, Tool, ToolDef, ToolExecutorError,
    },
    core::Message,
    models::ChatModel,
    providers::DeepSeekProvider,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use futures::StreamExt;

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

    // Create context with system prompt
    let context = Context::new()
        .layer(Layer::new(
            "system",
            LayerKind::System,
            serde_json::json!("You are a helpful assistant."),
        ))
        .layer(Layer::new(
            "conversation",
            LayerKind::Conversation,
            serde_json::json!([]),
        ));

    // Add user message to context
    let mut context = context;
    context.add_message(Message::user("What is the weather today?"));

    // Create agent actor
    let actor = AgentActorBuilder::new(model, tool_executor)
        .context(context)
        .max_iterations(10)
        .build();

    // Run loop and collect events
    let mut handle = actor.run_loop();

    // Process events in real-time
    while let Some(event) = handle.event_rx.recv().await {
        match event {
            AgentActorEvent::ContentChunk(text) => {
                print!("{}", text);
            }
            AgentActorEvent::ReasoningChunk(text) => {
                eprintln!("[Reasoning] {}", text);
            }
            AgentActorEvent::StepCompleted { content, tool_calls, .. } => {
                println!("\nStep completed: {}", content);
            }
            AgentActorEvent::StepFinalized { result } => {
                println!("Step finalized");
            }
            AgentActorEvent::ToolCalls(calls) => {
                println!("Tool calls: {:?}", calls.iter().map(|c| c.get_name()).collect::<Vec<_>>());
            }
            AgentActorEvent::ToolResult { call_id, success, output } => {
                println!("Tool {} completed: success={}", call_id, success);
            }
            AgentActorEvent::Iteration { iteration, message_count } => {
                println!("Iteration {} complete, {} messages", iteration, message_count);
            }
            AgentActorEvent::HookEvent { hook, kind, payload } => {
                println!("Hook event from {}: {}", hook, kind);
            }
            AgentActorEvent::Completed => {
                println!("Agent completed");
                break;
            }
            AgentActorEvent::Cancelled => {
                println!("Agent cancelled");
                break;
            }
            AgentActorEvent::Error(e) => {
                eprintln!("Error: {}", e);
                break;
            }
            AgentActorEvent::MaxIterations { iteration } => {
                println!("Max iterations reached: {}", iteration);
                break;
            }
        }
    }

    Ok(())
}
```

#### Manual Step Control

For fine-grained control over each iteration:

```rust
let mut actor = AgentActorBuilder::new(model, tool_executor)
    .context(context)
    .build();

loop {
    let result = actor.run_step(None).await;

    match result {
        StepResult::Continue { content, tools_call, tools_result, .. } => {
            println!("Step completed with tools, continuing...");
            // Access state: actor.state().context, actor.state().iteration
        }
        StepResult::Done { content, .. } => {
            println!("Agent finished: {}", content);
            break;
        }
        StepResult::Error(e) => {
            eprintln!("Error: {}", e);
            break;
        }
    }
}

// Access final state
let state = actor.state();
println!("Total iterations: {}", state.iteration);
println!("Final context: {:?}", state.context);
```

#### Control Handle

The `AgentActorHandle` provides async control:

```rust
let handle = actor.run_loop();

// Pause execution
handle.pause().await;

// Later, resume
handle.resume().await;

// Or cancel entirely
handle.cancel().await;

// Wait for completion
let events = handle.wait().await;
```

#### Architecture Notes

- `StepLifeCycle` is the core orchestrator that runs hooks at each `LifeCycle` phase
- Default hooks (`ExecutionPolicyHook`, `MetricsHook`, `TokenStatisticsHook`) are created in `StepLifeCycle::new()`
- To customize execution behavior, implement `LifeCycleHook` trait and inject via `StepLifeCycle::hooks`
- `call_model` parameter order: `(model, messages, tools_def)` - model first
- The module split: `agent_actor/mod.rs` (facade), `lifecycle.rs` (step execution), `loop_control.rs` (loop control), `builder.rs` (construction), `types.rs` (types)
