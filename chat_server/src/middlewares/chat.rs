use axum::{
    extract::{FromRequestParts, Path, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chat_core::User;

use crate::{AppError, AppState};

/// 验证Chats是否为当前用户所拥有
pub async fn verify_chat(State(state): State<AppState>, req: Request, next: Next) -> Response {
    // 从路径中获取chat_id
    let (mut parts, body) = req.into_parts();
    let Path(chat_id) = Path::<u64>::from_request_parts(&mut parts, &state)
        .await
        .unwrap();

    // 从请求中获取用户 需要确保verify_token先执行
    let user = parts.extensions.get::<User>().unwrap();
    // 验证用户是否为chat的成员
    if !state
        .is_chat_member(chat_id, user.id as _)
        .await
        .unwrap_or_default()
    {
        return AppError::CreateMessageError(format!(
            "user {} not a member of chat {}",
            user.id, chat_id
        ))
        .into_response();
    }

    // 重建请求
    let req = Request::from_parts(parts, body);

    next.run(req).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;
    use axum::{
        body::Body, http::StatusCode, middleware::from_fn_with_state, routing::get, Router,
    };

    use chat_core::middlewares::verify_token;
    use tower::ServiceExt;

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    #[tokio::test]
    async fn verify_chat_middleware_should_ok() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let user = state.find_user_by_id(1).await?.expect("user should exist");
        let token = state.ek.sign(user)?;

        let app = Router::new()
            .route("/chats/:user_id/messages", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_chat))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
            .with_state(state);

        // User in chat
        let req = Request::builder()
            .uri("/chats/1/messages")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        // User not in chat
        let req = Request::builder()
            .uri("/chats/1024/messages")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}
