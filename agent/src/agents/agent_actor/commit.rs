use tokio::sync::mpsc;

use super::AgentActorEvent;
use super::types::step_result_from_draft;
use crate::agents::hooks::{StepFrame, StepResultDraft};
use crate::agents::{AgentError, AgentState, JobState};
use crate::core::Message;

pub(super) struct CommitPlan {
    pub(super) next_state: JobState,
    pub(super) context_messages: Vec<Message>,
    pub(super) events: Vec<AgentActorEvent>,
}

pub(super) struct CommitReducer;

impl CommitReducer {
    pub(super) fn reduce(
        state: &AgentState,
        frame: &StepFrame,
        result: &StepResultDraft,
    ) -> CommitPlan {
        let context_messages = Self::context_messages(result);
        let message_count = state.context.conversation().len() + context_messages.len();
        let committed_result = step_result_from_draft(result.clone());

        let mut events = frame.pending_events.clone();
        events.push(AgentActorEvent::StepFinalized {
            result: committed_result.clone(),
        });

        match result {
            StepResultDraft::Continue { .. } | StepResultDraft::Done { .. } => {
                events.push(AgentActorEvent::Iteration {
                    iteration: state.iteration,
                    message_count,
                });
            }
            StepResultDraft::Error(AgentError::MaxIterations(iteration)) => {
                events.push(AgentActorEvent::MaxIterations {
                    iteration: *iteration,
                });
            }
            StepResultDraft::Error(AgentError::Cancelled) => {
                events.push(AgentActorEvent::Cancelled);
            }
            StepResultDraft::Error(err) => {
                events.push(AgentActorEvent::Error(err.clone()));
            }
        }

        CommitPlan {
            next_state: Self::next_state(result),
            context_messages,
            events,
        }
    }

    fn next_state(result: &StepResultDraft) -> JobState {
        match result {
            StepResultDraft::Continue { .. } => JobState::WaitingInput,
            StepResultDraft::Done { .. } => JobState::Completed,
            StepResultDraft::Error(AgentError::Cancelled) => JobState::Cancelled,
            StepResultDraft::Error(_) => JobState::Failed,
        }
    }

    fn context_messages(result: &StepResultDraft) -> Vec<Message> {
        match result {
            StepResultDraft::Continue {
                content,
                reasoning_content,
                tool_calls,
                tool_results,
            } => {
                let mut messages = vec![Message::Assistant {
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                }];
                messages.extend(tool_results.iter().map(|result| Message::Tool {
                    tool_call_id: result.call_id.clone(),
                    content: result.output.clone(),
                }));
                messages
            }
            StepResultDraft::Done {
                content,
                reasoning_content,
            } => vec![Message::Assistant {
                content: content.clone(),
                reasoning_content: reasoning_content.clone(),
                tool_calls: None,
            }],
            StepResultDraft::Error(_) => Vec::new(),
        }
    }
}

pub(super) struct StepCommitter;

impl StepCommitter {
    pub(super) async fn apply(
        plan: CommitPlan,
        state: &mut AgentState,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) {
        state.state = plan.next_state;
        for message in plan.context_messages {
            state.context.add_message(message);
        }

        if let Some(tx) = event_tx {
            for event in plan.events {
                let _ = tx.send(event).await;
            }
        }
    }
}
