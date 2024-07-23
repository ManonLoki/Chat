mod auth;
mod request_id;
mod server_time;

use std::fmt;

pub use auth::verify_token;
use axum::{middleware::from_fn, Router};
use server_time::ServerTimeLayer;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::User;

const RQEUST_ID_HEADER: &str = "x-request-id";

/// 验证Token
pub trait TokenVerify {
    /// Error 实现 Debug Trait
    type Error: fmt::Debug;
    /// 验证函数
    fn verify(&self, token: &str) -> Result<User, Self::Error>;
}

/// 配置Layer
pub fn set_layer(router: Router) -> Router {
    router.layer(
        ServiceBuilder::new()
            .layer(
                // 设置TracingLayer
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_request(DefaultOnRequest::new().level(Level::INFO))
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(tower_http::LatencyUnit::Micros),
                    ),
            )
            // 设置 Gzip
            .layer(CompressionLayer::new().gzip(true).br(true).deflate(true))
            // 设置RequestId
            .layer(from_fn(request_id::set_request_id))
            // 设置接口执行时间
            .layer(ServerTimeLayer),
    )
}
