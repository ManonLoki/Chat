mod config;
mod error;
mod handlers;
mod models;
pub use error::AppError;
use handlers::*;
pub use models::User;
use std::{ops::Deref, sync::Arc};

use axum::{
    routing::{get, patch, post},
    Router,
};
pub use config::AppConfig;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    inner: Arc<AppStateInner>,
}
#[allow(unused)]
#[derive(Debug)]
pub(crate) struct AppStateInner {
    pub config: AppConfig,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            inner: Arc::new(AppStateInner { config }),
        }
    }
}

impl Deref for AppState {
    type Target = Arc<AppStateInner>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub fn get_router(config: AppConfig) -> Router {
    let state = AppState::new(config);
    let api = Router::new()
        .route("/signin", post(signin_handler))
        .route("/signup", post(singup_handler))
        .route("/chat", get(list_chat_handler).post(create_chat_handler))
        .route(
            "/chat/:id",
            patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/chat/:id/messages", get(list_message_handler));
    Router::new()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state)
}
