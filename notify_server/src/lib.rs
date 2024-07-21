mod config;
mod error;
mod notify;
mod sse;

use std::{ops::Deref, sync::Arc};

use axum::{middleware::from_fn_with_state, response::IntoResponse, routing::get, Router};
use axum_extra::response::Html;
use chat_core::{
    middlewares::{verify_token, TokenVerify},
    utils::DecodingKey,
};

pub use config::AppConfig;
use dashmap::DashMap;

pub use notify::{setup_pg_listener, AppEvent};
use sse::sse_handler;
use tokio::sync::broadcast;

pub type UserMap = Arc<DashMap<u64, broadcast::Sender<Arc<AppEvent>>>>;

const INDEX_HTML: &str = include_str!("../index.html");

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

pub struct AppStateInner {
    pub config: AppConfig,
    pub users: UserMap,
    dk: DecodingKey,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let dk = DecodingKey::load(&config.auth.pk).expect("Failed to load decoding key");
        Self(Arc::new(AppStateInner {
            config,
            dk,
            users: Arc::new(DashMap::new()),
        }))
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TokenVerify for AppState {
    type Error = jwt_simple::Error;

    fn verify(&self, token: &str) -> Result<chat_core::User, Self::Error> {
        self.dk.verify(token)
    }
}

pub async fn get_router(config: AppConfig) -> anyhow::Result<Router> {
    let state = AppState::new(config);

    setup_pg_listener(state.clone()).await?;

    let router = Router::new()
        .route("/events", get(sse_handler))
        .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
        .route("/", get(index_handler))
        .with_state(state.clone());

    Ok(router)
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}
