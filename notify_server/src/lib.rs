use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::response::Html;
use sse::sse_handler;
mod sse;

const INDEX_HTML: &str = include_str!("../index.html");

pub fn get_router() -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/events", get(sse_handler))
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}
