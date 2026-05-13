use std::collections::VecDeque;

use bevy_ecs::resource::Resource;

use crate::turn_messages::PlayerCommand;

#[derive(Resource, Debug, Default)]
pub struct PlayerInbox {
    commands: VecDeque<PlayerCommand>,
}

impl PlayerInbox {
    pub fn push(&mut self, command: PlayerCommand) {
        self.commands.push_back(command);
    }

    pub fn pop(&mut self) -> Option<PlayerCommand> {
        self.commands.pop_front()
    }
}
