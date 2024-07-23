use axum::http::StatusCode;
use axum::response::Json;
use axum::response::{IntoResponse, Response};
use jwt_simple::reexports::serde_json::json;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// 封装错误输出
#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct ErrorOutput {
    pub error: String,
}

impl ErrorOutput {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

/// 应用的错误上下文
#[derive(Error, Debug)]
pub enum AppError {
    /// SqlX的错误
    #[error("sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    /// Argon2的错误
    #[error("password hash  error: {0}")]
    PasswordHashError(#[from] argon2::password_hash::Error),
    /// Jwt的错误
    #[error("jwt error: {0}")]
    JWTError(#[from] jwt_simple::Error),
    /// Http Header的错误
    #[error("http header parse error:{0}")]
    HttpHeaderError(#[from] axum::http::header::InvalidHeaderValue),
    /// Email已存在(自定义)
    #[error("email already exists:{0}")]
    EmailAlreadyExists(String),
    /// 创建聊天室错误(自定义)
    #[error("create chat error:{0}")]
    CreateChatError(String),
    /// 数据不存在(自定义)
    #[error("not found {0}")]
    NotFound(String),
    /// 数据未改变(自定义)
    #[error("not change {0}")]
    NotChange(String),
    /// Tokio的IO错误
    #[error("io error:{0}")]
    IoError(#[from] tokio::io::Error),
    /// 未授权(自定义)
    #[error("unauthorized")]
    Unauthorized,
    /// 创建消息错误(自定义)
    #[error("create message error:{0}")]
    CreateMessageError(String),
    /// 聊天文件错误(自定义)
    #[error("{0}")]
    ChatFileError(String),
}

/// 实现IntoResponse 这样才符合Infallible
impl IntoResponse for AppError {
    fn into_response(self) -> Response<axum::body::Body> {
        let status = match self {
            AppError::SqlxError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::PasswordHashError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::JWTError(_) => StatusCode::FORBIDDEN,
            AppError::HttpHeaderError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::EmailAlreadyExists(_) => StatusCode::CONFLICT,
            AppError::CreateChatError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::NotChange(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::CreateMessageError(_) => StatusCode::BAD_REQUEST,
            AppError::ChatFileError(_) => StatusCode::BAD_REQUEST,
        };

        (status, Json(json!({"error":self.to_string()}))).into_response()
    }
}
