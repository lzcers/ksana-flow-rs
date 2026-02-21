/// 用量
#[derive(Debug, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
/// 消息角色
#[derive(Debug, Clone)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 内容片段：支持多模态
/// 扩展性体现：新增模态只需在此添加 Variant，无需修改上层逻辑
#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    ImageUrl { url: String, detail: Option<String> },
    // InputAudio {  Vec<u8>, format: String },
}

/// 标准消息结构
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>, // 使用 Vec 支持混合内容 (如：文本 + 图片)
    pub name: Option<String>,  // 用于标识特定用户或工具
}
