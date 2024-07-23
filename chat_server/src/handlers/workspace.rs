use axum::{extract::State, response::IntoResponse, Extension, Json};

use crate::{AppError, AppState, User};

/// 获取workspace下的所有用户
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of workspace users", body = Vec<User>),
    ),
    security(
        ("token" = [])
    ),
    tag = "users"
)]
pub(crate) async fn list_chat_users_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let users = state.fetch_chat_users(user.ws_id as _).await?;

    Ok(Json(users))
}
