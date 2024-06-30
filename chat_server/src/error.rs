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
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<axum::body::Body> {
        let status = match self {
            AppError::SqlxError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::PasswordHashError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::JWTError(_) => StatusCode::FORBIDDEN,
            AppError::HttpHeaderError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::EmailAlreadyExists(_) => StatusCode::CONFLICT,
        };

        (status, Json(json!({"error":self.to_string()}))).into_response()
    }
}
