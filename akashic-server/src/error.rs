use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub success: bool,
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AppError {
    pub fn not_implemented(endpoint: &'static str) -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            code: "NOT_IMPLEMENTED",
            message: format!("接口 `{endpoint}` 已定义，但尚未实现业务逻辑"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            success: false,
            error: ErrorDetail {
                code: self.code.to_string(),
                message: self.message,
            },
        };

        (self.status, Json(body)).into_response()
    }
}
