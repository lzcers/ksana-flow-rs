use crate::resources::protagonist_action::PlayerActionInput;

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    SubmitPlayerAction {
        turn_id: u64,
        input: PlayerActionInput,
    },
}
