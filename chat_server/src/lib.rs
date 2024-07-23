mod config;
mod error;
mod handlers;
mod middlewares;
mod models;
mod openapi;

use anyhow::Context;
use chat_core::{
    middlewares::{set_layer, verify_token, TokenVerify},
    utils::{DecodingKey, EncodingKey},
    User,
};
pub use error::AppError;
use handlers::*;
use middlewares::verify_chat;
use openapi::OpenApiRouter;
use std::{fmt::Debug, ops::Deref, sync::Arc};

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
pub use config::AppConfig;

/// AppState 上下文要保证可以被Clone
#[derive(Debug, Clone)]
pub struct AppState {
    /// 内部使用Arc保证了数据可以最小成本Clone
    inner: Arc<AppStateInner>,
}

/// 上下文内容
pub struct AppStateInner {
    /// 配置
    pub(crate) config: AppConfig,
    /// Token加密Key
    pub(crate) ek: EncodingKey,
    /// Token解密Key
    pub(crate) dk: DecodingKey,
    /// 数据库连接池
    pub(crate) pool: sqlx::PgPool,
}

/// 实现TokenVerify trait
impl TokenVerify for AppState {
    type Error = AppError;
    fn verify(&self, token: &str) -> Result<User, Self::Error> {
        let user = self.dk.verify(token).context("decode token failed")?;
        Ok(user)
    }
}

/// 实现AppState
impl AppState {
    /// 创建一个新的AppState 可失败
    pub async fn try_new(config: AppConfig) -> Result<Self, AppError> {
        let ek = EncodingKey::load(&config.auth.sk).context("load sk failed")?;
        let dk = DecodingKey::load(&config.auth.pk).context("load pk failed")?;
        let pool = sqlx::PgPool::connect(&config.server.db_url)
            .await
            .context("load db error")?;
        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                ek,
                dk,
                pool,
            }),
        })
    }
}

/// 实现Deref 访问内部数据
impl Deref for AppState {
    type Target = Arc<AppStateInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// 实现Debug Trait
impl Debug for AppStateInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .finish()
    }
}

/// 获取路由
pub async fn get_router(state: AppState) -> Result<Router, AppError> {
    // 聊天室路由
    // 2 verify_chat middleware
    let chat = Router::new()
        .route(
            "/:id",
            get(get_chat_handler)
                .patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/:id/messages", get(list_message_handler))
        .layer(from_fn_with_state(state.clone(), verify_chat))
        .route("/", get(list_chat_handler).post(create_chat_handler));

    // API 路由
    // 1 token_verify middleware
    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .route("/upload", post(upload_handler))
        .route("/files/:ws_id/*path", get(file_handler))
        .nest("/chats", chat)
        .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler));

    // App 根路由
    let app = Router::new()
        // 设置OpenAPIApi路由
        .openapi()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state);

    Ok(set_layer(app))
}

#[cfg(feature = "test-util")]
mod test_util {
    use std::path::Path;

    use super::*;

    use sqlx::Executor;
    use sqlx::PgPool;
    use sqlx_db_tester::TestPg;

    impl AppState {
        pub async fn new_for_test() -> Result<(TestPg, Self), AppError> {
            let config = AppConfig::load().context("load config failed")?;
            println!("config: {:?}", config);
            tokio::fs::create_dir_all(&config.server.base_dir).await?;
            println!("created");
            let ek = EncodingKey::load(&config.auth.sk).context("load sk failed")?;
            let dk = DecodingKey::load(&config.auth.pk).context("load pk failed")?;

            let last_index = config.server.db_url.rfind('/').unwrap();
            let server_url = &config.server.db_url[..last_index];

            println!("server_url next: {}", server_url);
            let (tdb, pool) = get_test_pool(Some(server_url)).await;

            let state = Self {
                inner: Arc::new(AppStateInner {
                    config,
                    ek,
                    dk,
                    pool,
                }),
            };
            Ok((tdb, state))
        }
    }

    pub async fn get_test_pool(url: Option<&str>) -> (TestPg, PgPool) {
        let url = url.unwrap_or("postgres://postgres:880914@localhost:5432/");

        let tdb = TestPg::new(url.to_string(), Path::new("../migrations"));

        let pool = tdb.get_pool().await;

        let sql = include_str!("../fixtures/test.sql").split(';');

        let mut ts = pool.begin().await.expect("begin transaction failed");

        for s in sql {
            if s.trim().is_empty() {
                continue;
            }

            ts.execute(s).await.expect("execute failed");
        }

        ts.commit().await.expect("commit failed");

        (tdb, pool)
    }
}
