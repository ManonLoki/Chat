use axum::http::StatusCode;
use axum::response::Json;
use axum::response::{IntoResponse, Response};
use jwt_simple::reexports::serde_json::json;

use thiserror::Error;

/// 应用错误
#[derive(Error, Debug)]
pub enum AppError {
    /// Sqlx错误
    #[error("sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    /// JWT错误
    #[error("jwt error: {0}")]
    JWTError(#[from] jwt_simple::Error),
}

/// 将AppError转换为Response
impl IntoResponse for AppError {
    fn into_response(self) -> Response<axum::body::Body> {
        let status = match self {
            AppError::SqlxError(_) => StatusCode::INTERNAL_SERVER_ERROR,

            AppError::JWTError(_) => StatusCode::FORBIDDEN,
        };

        (status, Json(json!({"error":self.to_string()}))).into_response()
    }
}
