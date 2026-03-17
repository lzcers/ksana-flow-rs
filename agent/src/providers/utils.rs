use serde_json::Value;

use super::types::ProviderError;

/// 解析 SSE 格式的流数据行
///
/// SSE 格式为 "data: {json}" 或 "data: [DONE]"
pub fn parse_sse_line(line: &str) -> Option<Value> {
    let line = line.trim();
    if line.is_empty() || line == "data: [DONE]" {
        return None;
    }
    if let Some(data) = line.strip_prefix("data: ") {
        return serde_json::from_str(data).ok();
    }
    None
}

/// 解析 API 错误响应
///
/// 尝试从响应体中提取错误代码和消息
pub fn parse_api_error(body: &str, status: u16) -> ProviderError {
    if let Ok(error_json) = serde_json::from_str::<Value>(body) {
        let code = error_json["error"]["code"]
            .as_i64()
            .or_else(|| error_json["error"]["type"].as_str().map(|t| t.len() as i64))
            .unwrap_or(0) as u16;
        let message = error_json["error"]["message"]
            .as_str()
            .or_else(|| error_json["error"].as_str())
            .unwrap_or(body)
            .to_string();
        return ProviderError::ApiError { code, message };
    }
    ProviderError::ApiError {
        code: status,
        message: body.to_string(),
    }
}
