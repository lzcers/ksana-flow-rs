mod conversation;
mod rule;
mod summary;
mod types;

use crate::agent::Context;
use crate::agent::MemoryStore;

pub use types::{
    ChatSummaryModel, CompressionError, ConversationRule, LayerAction, LayerRule, LayerSelector,
    ModelCompression, RuleCompression, SummaryModel,
};

impl Context {
    pub fn compress_by_rule(&self, rule: &RuleCompression) -> Result<Self, CompressionError> {
        rule::compress_by_rule(self, rule, None)
    }

    pub fn compress_by_rule_with_archive(
        &self,
        rule: &RuleCompression,
        memory: &mut dyn MemoryStore,
    ) -> Result<Self, CompressionError> {
        rule::compress_by_rule(self, rule, Some(memory))
    }

    pub async fn compress_by_model(
        &self,
        model: &dyn SummaryModel,
        options: &ModelCompression,
    ) -> Result<Self, CompressionError> {
        summary::compress_by_model(self, model, options).await
    }
}

#[cfg(test)]
mod tests;
