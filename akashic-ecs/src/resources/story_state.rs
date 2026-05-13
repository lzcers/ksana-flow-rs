use bevy_ecs::resource::Resource;

/// 最近一次生成完成的叙事正文。
#[derive(Resource, Debug, Clone, Default)]
pub struct LatestNarration(pub String);

impl LatestNarration {
    pub fn get(&self) -> &str {
        &self.0
    }

    pub fn set(&mut self, content: String) {
        self.0 = content;
    }
}
