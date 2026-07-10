use super::*;
use crate::{
    Controller, ControllerRunners, Graph, Input, Node, Output, RunnerKind, flow::runner::NodeState,
};
use async_trait::async_trait;
use futures::{future, stream};
use std::time::Duration;
use tokio::time::timeout;

async fn collect_until_terminal(stream: ReactiveStream) -> Vec<TaskEvent> {
    let (tx, mut rx) = mpsc::channel(8);
    let task = (stream.start)(
        TaskGuard::default(),
        tx,
        "source".to_string(),
        Arc::new(Context::new()),
    );
    let mut events = Vec::new();

    while let Some(event) = rx.recv().await {
        let terminal = matches!(event, TaskEvent::Completed(..) | TaskEvent::Error(..));
        events.push(event);
        if terminal {
            break;
        }
    }

    task.abort();
    events
}

#[tokio::test]
async fn stream_forwards_each_item_then_completes() {
    let source = stream::iter([Ok::<_, ()>(1), Ok(2)]);
    let events = collect_until_terminal(ReactiveStream::from_stream(source)).await;

    assert!(matches!(&events[0], TaskEvent::Next(id, value) if id == "source" && value == 1));
    assert!(matches!(&events[1], TaskEvent::Next(id, value) if id == "source" && value == 2));
    assert!(matches!(&events[2], TaskEvent::Completed(id, None) if id == "source"));
}

#[tokio::test]
async fn stream_accumulator_produces_final_output() {
    let source = stream::iter([Ok::<_, ()>("a".to_string()), Ok("b".to_string())]);
    let events = collect_until_terminal(ReactiveStream::from_stream_with_accumulator(
        source,
        |items| Some(Value::String(items.concat())),
    ))
    .await;

    assert!(matches!(&events[0], TaskEvent::Next(id, value) if id == "source" && value == "a"));
    assert!(matches!(&events[1], TaskEvent::Next(id, value) if id == "source" && value == "b"));
    assert!(matches!(
        &events[2],
        TaskEvent::Completed(id, Some(Value::String(value)))
            if id == "source" && value == "ab"
    ));
}

#[tokio::test]
async fn stream_forwards_source_errors() {
    let source = stream::iter([Ok(1), Err(()), Ok(2)]);
    let events = collect_until_terminal(ReactiveStream::from_stream(source)).await;

    assert!(matches!(&events[0], TaskEvent::Next(id, value) if id == "source" && value == 1));
    assert!(matches!(
        &events[1],
        TaskEvent::Error(id, error) if id == "source" && error == "Stream error"
    ));
    assert_eq!(events.len(), 2, "stream errors must be terminal");
}

#[tokio::test]
async fn abort_cancels_the_stream_task() {
    let stream = ReactiveStream::from_stream(stream::pending::<Result<(), ()>>());
    let (tx, mut rx) = mpsc::channel(1);
    let task = (stream.start)(
        TaskGuard::default(),
        tx,
        "source".to_string(),
        Arc::new(Context::new()),
    );

    task.abort();

    assert!(
        timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("cancelled stream should close its event channel")
            .is_none()
    );
}

struct FiniteStreamNode;

#[async_trait]
impl Node for FiniteStreamNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        let mut output = Output::new(None);
        output.set_stream(ReactiveStream::from_stream(stream::iter([
            Ok::<_, ()>(1),
            Ok(2),
        ])));
        Ok(output)
    }
}

#[tokio::test]
async fn runner_completes_after_consuming_a_finite_stream() {
    let mut graph = Graph::new();
    graph.add_node("source", || FiniteStreamNode);
    let (controller, _events) = Controller::new();
    let (_runner_id, mut runner, _handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None, None);
    runner.set_start_node("source", Value::Null.into());

    runner.run().await.expect("finite stream should complete");

    assert_eq!(
        runner.get_execution_context().get_state("source"),
        Some(NodeState::Completed)
    );
    assert_eq!(
        runner.get_execution_context().get_output("source"),
        Some(Value::from(2))
    );
}

struct ErrorStreamNode;

#[async_trait]
impl Node for ErrorStreamNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        let mut output = Output::new(None);
        output.set_stream(ReactiveStream::from_stream(stream::once(async {
            Err::<(), _>(())
        })));
        Ok(output)
    }
}

#[tokio::test]
async fn runner_fails_when_a_stream_emits_an_error() {
    let mut graph = Graph::new();
    graph.add_node("source", || ErrorStreamNode);
    let (controller, _events) = Controller::new();
    let (_runner_id, mut runner, _handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None, None);
    runner.set_start_node("source", Value::Null.into());

    let error = runner
        .run()
        .await
        .expect_err("stream error should fail runner");

    assert_eq!(error, "Stream error");
    assert_eq!(
        runner.get_execution_context().get_state("source"),
        Some(NodeState::Failed)
    );
}

struct PendingStreamNode {
    started: Arc<tokio::sync::Notify>,
    dropped: Arc<tokio::sync::Notify>,
}

struct DropSignal(Arc<tokio::sync::Notify>);

impl Drop for DropSignal {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

#[async_trait]
impl Node for PendingStreamNode {
    async fn run(&mut self, _ctx: &Context, _input: &Input) -> Result<Output, String> {
        let started = self.started.clone();
        let drop_signal = DropSignal(self.dropped.clone());
        let stream = ReactiveStream::new(move |guard, _tx, _node_id, _ctx| async move {
            let _guard = guard;
            let _drop_signal = drop_signal;
            started.notify_one();
            future::pending().await
        });
        let mut output = Output::new(None);
        output.set_stream(stream);
        Ok(output)
    }
}

#[tokio::test]
async fn runner_stop_cancels_an_active_stream() {
    let started = Arc::new(tokio::sync::Notify::new());
    let dropped = Arc::new(tokio::sync::Notify::new());
    let mut graph = Graph::new();
    graph.add_node("source", {
        let started = started.clone();
        let dropped = dropped.clone();
        move || PendingStreamNode {
            started: started.clone(),
            dropped: dropped.clone(),
        }
    });
    let (controller, _events) = Controller::new();
    let (runner_id, mut runner, handle) =
        controller.create_runner(Arc::new(graph), None, RunnerKind::Root, None, None);
    runner.set_start_node("source", Value::Null.into());
    let runner_task = controller.spawn_runner(runner_id, runner);

    timeout(Duration::from_secs(1), started.notified())
        .await
        .expect("stream producer should start");
    let dropped_signal = dropped.notified();
    handle.stop().await;

    timeout(Duration::from_secs(1), dropped_signal)
        .await
        .expect("stopping the runner should drop the stream producer");
    timeout(Duration::from_secs(1), runner_task)
        .await
        .expect("stopped runner should terminate")
        .expect("runner task should not panic")
        .expect("runner should stop cleanly");
}
