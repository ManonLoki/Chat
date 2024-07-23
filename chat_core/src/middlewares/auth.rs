use axum::{
    extract::{FromRequestParts, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use serde::Deserialize;

use super::TokenVerify;

/// Url参数 ?access_token=xxx
#[derive(Debug, Deserialize)]
struct Params {
    access_token: String,
}

// 校验Token
pub async fn verify_token<T>(State(state): State<T>, req: Request, next: Next) -> Response
where
    T: TokenVerify + Clone + Send + Sync + 'static,
{
    // 将request转换为 Request Parts和Body
    let (mut parts, body) = req.into_parts();

    // 从Parts中拿到Authorization Header
    let token =
        match TypedHeader::<Authorization<Bearer>>::from_request_parts(&mut parts, &state).await {
            Ok(TypedHeader(Authorization(bearer))) => bearer.token().to_string(),
            Err(e) => {
                // 获取失败的情况下，尝试从Query中获取
                if e.is_missing() {
                    // 如果实现一个自己的Extra 也需要实现FromRequestParts
                    match Query::<Params>::from_request_parts(&mut parts, &state).await {
                        Ok(Query(params)) => params.access_token,
                        Err(e) => {
                            let msg = format!("Failed to parse query params: {:?}", e);
                            tracing::warn!(msg);
                            return (StatusCode::UNAUTHORIZED, msg).into_response();
                        }
                    }
                } else {
                    let msg = format!("Failed to parse Authorization header: {:?}", e);
                    tracing::warn!(msg);
                    return (StatusCode::UNAUTHORIZED, msg).into_response();
                }
            }
        };

    // 验证Token
    let req = match state.verify(&token) {
        Ok(user) => {
            // 成功的话 将Parts和Body重新组装成Request
            let mut req = Request::from_parts(parts, body);
            // 向Extension总插入User
            req.extensions_mut().insert(user);
            req
        }
        Err(e) => {
            let msg = format!("Failed to verify token: {:?}", e);
            tracing::warn!(msg);
            return (StatusCode::FORBIDDEN, msg).into_response();
        }
    };

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        utils::{DecodingKey, EncodingKey},
        User,
    };

    use super::*;
    use anyhow::Result;
    use axum::{body::Body, middleware::from_fn_with_state, routing::get, Router};

    use tower::ServiceExt;

    // 创建Mock的AppState
    #[derive(Clone)]
    struct AppState(Arc<AppStateInner>);
    struct AppStateInner {
        ek: EncodingKey,
        dk: DecodingKey,
    }

    impl TokenVerify for AppState {
        type Error = jwt_simple::Error;

        fn verify(&self, token: &str) -> Result<User, Self::Error> {
            self.0.dk.verify(token)
        }
    }

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    #[tokio::test]
    async fn verify_token_middleware_should_ok() -> Result<()> {
        let encoding_pem = include_str!("../../fixtures/encoding.pem");
        let decoding_pem = include_str!("../../fixtures/decoding.pem");
        let ek = EncodingKey::load(encoding_pem)?;
        let dk = DecodingKey::load(decoding_pem)?;
        let state = AppState(Arc::new(AppStateInner { dk, ek }));

        let user = User::new(1, "Manonloki", "manonloki@gmail.com");
        let token = state.0.ek.sign(user)?;

        let app = Router::new()
            .route("/api", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
            .with_state(state);
        // good token
        let req = Request::builder()
            .uri("/api")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        let req = Request::builder()
            .uri(format!("/api?access_token={}", token))
            .body(Body::empty())?;

        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        // Bad Token
        let req = Request::builder().uri("/api").body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }
}
