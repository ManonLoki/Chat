use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};

use crate::{
    models::{CreateChat, UpdateChat},
    AppError, AppState, User,
};

/// 获取所有聊天室
#[utoipa::path(
    get,
    path = "/api/chats",
    responses(
        (status = 200, description = "List of all chats", body = Vec<Chat>),
    ),
    security(
        ("token" = [])
    ),
    tag = "chat"
)]
pub(crate) async fn list_chat_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let chats = state.fetch_all_chat(user.ws_id as _).await?;
    Ok((StatusCode::OK, Json(chats)))
}

/// 创建聊天室
#[utoipa::path(
    post,
    path = "/api/chats",
    responses(
        (status = 201, description = "Chat created", body = Chat),
    ),
    security(
        ("token" = [])
    ),
    tag = "chat"
)]
pub(crate) async fn create_chat_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Json(input): Json<CreateChat>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.create_chat(input, user.ws_id as _).await?;
    Ok((StatusCode::CREATED, Json(chat)))
}

/// 根据ID获取聊天室
#[utoipa::path(
    get,
    path = "/api/chats/{id}",
    params(
        ("id" = u64, Path, description = "Get Chat Detail By ID")
    ),
    responses(
        (status = 200, description = "Chat found", body = Chat),
        (status = 404, description = "Chat not found", body = ErrorOutput),
    ),
    security(
        ("token" = [])
    ),
    tag = "chat"
)]
pub(crate) async fn get_chat_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    let chat = state.get_chat_by_id(id as _).await?;
    match chat {
        Some(chat) => Ok((StatusCode::OK, Json(chat))),
        None => Err(AppError::NotFound(format!("chat id {}", id))),
    }
}

/// 更新聊天室信息
#[utoipa::path(
    patch,
    path = "/api/chats/{id}",
    responses(
        (status = 200, description = "Update Chat Info"),
    ),
    security(
        ("token" = [])
    ),
    tag = "chat"
)]
pub(crate) async fn update_chat_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<UpdateChat>,
) -> Result<impl IntoResponse, AppError> {
    state.update_chat_by_id(id, input).await?;

    Ok(StatusCode::OK)
}

/// 删除聊天室
#[utoipa::path(
    delete,
    path = "/api/chats/{id}",
    responses(
        (status = 200, description = "Remove Chat"),
    ),
    security(
        ("token" = [])
    ),
    tag = "chat"
)]
pub(crate) async fn delete_chat_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse, AppError> {
    state.delete_chat_by_id(id as _).await?;
    Ok(StatusCode::OK)
}
