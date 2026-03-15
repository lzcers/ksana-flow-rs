mod deepseek;
mod openai_compatible;
mod openrouter;
mod types;
mod utils;

// 重导出公共接口
pub use deepseek::{DeepSeekProvider, deepseek_provider, deepseek_provider_from_env};
pub use openai_compatible::OpenAICompatibleProvider;
pub use openrouter::{
    OpenRouterProvider, openrouter_provider, openrouter_provider_from_env,
    openrouter_provider_with_config,
};
pub use types::*;
pub use utils::{parse_api_error, parse_sse_line};
