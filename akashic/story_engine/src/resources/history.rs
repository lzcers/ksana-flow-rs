use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};

use crate::resources::{
    protagonist_action::PendingProtagonistChoice, world_snapshot::WorldSnapshot,
};

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionHistoryLog {
    pub rounds: Vec<RoundHistoryEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundHistoryEntry {
    pub round: u64,
    pub world_snapshot: Option<WorldSnapshot>,
    pub narration_text: Option<String>,
    pub choices: Vec<PendingProtagonistChoice>,
    pub committed_action: Option<String>,
}

impl SessionHistoryLog {
    pub fn ensure_round_mut(&mut self, round: u64) -> &mut RoundHistoryEntry {
        if let Some(index) = self.rounds.iter().position(|entry| entry.round == round) {
            return &mut self.rounds[index];
        }

        self.rounds.push(RoundHistoryEntry {
            round,
            ..RoundHistoryEntry::default()
        });
        self.rounds.sort_by_key(|entry| entry.round);
        self.rounds
            .iter_mut()
            .find(|entry| entry.round == round)
            .expect("刚插入的轮次必须存在")
    }

    pub fn set_world_snapshot(&mut self, round: u64, snapshot: WorldSnapshot) {
        self.ensure_round_mut(round).world_snapshot = Some(snapshot);
    }

    pub fn set_narration(&mut self, round: u64, text: String) {
        self.ensure_round_mut(round).narration_text = Some(text);
    }

    pub fn set_choices(&mut self, round: u64, choices: Vec<PendingProtagonistChoice>) {
        self.ensure_round_mut(round).choices = choices;
    }

    pub fn set_committed_action(&mut self, round: u64, action: String) {
        self.ensure_round_mut(round).committed_action = Some(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_round_updates_isolated_and_sorted() {
        let mut history = SessionHistoryLog::default();

        history.set_narration(2, "second".to_string());
        history.set_narration(1, "first".to_string());
        history.set_committed_action(2, "continue".to_string());

        assert_eq!(
            history
                .rounds
                .iter()
                .map(|round| round.round)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(history.rounds[0].narration_text.as_deref(), Some("first"));
        assert_eq!(history.rounds[1].narration_text.as_deref(), Some("second"));
        assert_eq!(
            history.rounds[1].committed_action.as_deref(),
            Some("continue")
        );
    }
}
