mod config;
mod error;
mod handlers;
mod middlewares;
mod models;
mod utils;
use anyhow::Context;
pub use error::AppError;
use handlers::*;
use middlewares::{set_layer, verify_token};
pub use models::User;
use std::{fmt::Debug, ops::Deref, sync::Arc};

use utils::{DecodingKey, EncodingKey};

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
pub use config::AppConfig;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    inner: Arc<AppStateInner>,
}
#[allow(unused)]
pub(crate) struct AppStateInner {
    pub(crate) config: AppConfig,
    pub(crate) ek: EncodingKey,
    pub(crate) dk: DecodingKey,
    pub(crate) pool: sqlx::PgPool,
}

impl AppState {
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

impl Deref for AppState {
    type Target = Arc<AppStateInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Debug for AppStateInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .finish()
    }
}

pub async fn get_router(config: AppConfig) -> Result<Router, AppError> {
    let state = AppState::try_new(config).await?;
    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .route("/chats", get(list_chat_handler).post(create_chat_handler))
        .route(
            "/chats/:id",
            get(get_chat_handler)
                .patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/chats/:id/messages", get(list_message_handler))
        .route("/upload", post(upload_handler))
        .route("/files/:ws_id/*path", get(file_handler))
        .layer(from_fn_with_state(state.clone(), verify_token))
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler));

    let app = Router::new()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state);

    Ok(set_layer(app))
}

#[cfg(test)]
mod test_util {
    use std::path::Path;

    use super::*;

    use sqlx::Executor;
    use sqlx::PgPool;
    use sqlx_db_tester::TestPg;

    impl AppState {
        pub async fn new_for_test(config: AppConfig) -> Result<(TestPg, Self), AppError> {
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
