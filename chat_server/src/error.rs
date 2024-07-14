use axum::http::StatusCode;
use axum::response::Json;
use axum::response::{IntoResponse, Response};
use jwt_simple::reexports::serde_json::json;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("password hash  error: {0}")]
    PasswordHashError(#[from] argon2::password_hash::Error),
    #[error("jwt error: {0}")]
    JWTError(#[from] jwt_simple::Error),

    #[error("http header parse error:{0}")]
    HttpHeaderError(#[from] axum::http::header::InvalidHeaderValue),
    #[error("email already exists:{0}")]
    EmailAlreadyExists(String),

    #[error("create chat error:{0}")]
    CreateChatError(String),

    #[error("not found {0}")]
    NotFound(String),
    #[error("not change {0}")]
    NotChange(String),

    #[error("io error:{0}")]
    IoError(#[from] tokio::io::Error),

    #[error("unauthorized")]
    Unauthorized,

    #[error("create message error:{0}")]
    CreateMessageError(String),

    #[error("{0}")]
    ChatFileError(String),
}

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
