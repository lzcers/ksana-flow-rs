use std::collections::HashMap;

use serde_json::Value;

use super::{ModelCallOutput, StepResultDraft, StepScratchpad};
use crate::agents::{CallToolResult, ToolCall};

#[derive(Default)]
pub(crate) struct StepFrame {
    pub metadata: HashMap<String, Value>,
    pub scratchpad: StepScratchpad,
    pub model_output: ModelCallOutput,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<CallToolResult>,
    pub final_result: Option<StepResultDraft>,
}

impl StepFrame {
    pub(crate) fn result(&self) -> Option<&StepResultDraft> {
        self.final_result.as_ref()
    }

    pub(crate) fn set_result(&mut self, result: StepResultDraft) {
        self.final_result = Some(result);
    }
}
