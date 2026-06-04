use bevy_ecs::component::Component;

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorySession {
    pub id: String,
}

#[derive(Component, Debug, Clone)]
pub struct SessionProfiles {
    pub world_profile: String,
    pub protagonist_profile: String,
    pub key_story_beats: String,
}
