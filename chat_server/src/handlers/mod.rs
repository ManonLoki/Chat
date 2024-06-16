mod auth;
mod chat;
mod message;

pub(crate) use auth::*;
use axum::response::IntoResponse;
pub(crate) use chat::*;
pub(crate) use message::*;

pub(crate) async fn index_handler() -> impl IntoResponse {
    "Hello, World!"
}
