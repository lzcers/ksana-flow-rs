use bevy_ecs::component::Component;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInputConfig {
    pub auto_select_first: bool,
}

impl PlayerInputConfig {
    pub const fn wait_for_user() -> Self {
        Self {
            auto_select_first: false,
        }
    }

    pub const fn auto_select_first() -> Self {
        Self {
            auto_select_first: true,
        }
    }
}

impl Default for PlayerInputConfig {
    fn default() -> Self {
        Self::wait_for_user()
    }
}
