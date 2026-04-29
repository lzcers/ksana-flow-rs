use bevy_ecs::resource::Resource;

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ManualControlState {
    pub pending_protagonist_override_turn: Option<u64>,
}
