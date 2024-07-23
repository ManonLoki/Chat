use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::RQEUST_ID_HEADER;
use axum::{extract::Request, response::Response};
use tokio::time::Instant;
use tower::{Layer, Service};

/// 接口执行时Layer
#[derive(Clone)]
pub struct ServerTimeLayer;

/// 实现Layer Trait 返回一个Middlerware市里
impl<S> Layer<S> for ServerTimeLayer {
    type Service = ServerTimeMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ServerTimeMiddleware { inner }
    }
}

/// MiddleWare
#[derive(Clone)]
pub struct ServerTimeMiddleware<S> {
    // 来自外部的Service
    inner: S,
}

/// 实现ServiceTrait
impl<S> Service<Request> for ServerTimeMiddleware<S>
where
    // Serverice需要实现Service, Send, 并且拥有所有权
    S: Service<Request, Response = Response> + Send + 'static,
    // 同理 返回的Future也需要实现 Send, 并且拥有所有权
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // 调用时创建一个时间点
        let start = Instant::now();
        // 等待内部Service调用完成
        let future = self.inner.call(request);
        Box::pin(async move {
            // 获取Response
            let mut response: Response = future.await?;

            // 计算时间
            let elapsed = format!("{}us", start.elapsed().as_micros());

            match elapsed.parse() {
                Ok(v) => {
                    // 加入到Header中
                    response.headers_mut().insert("x-server-time", v);
                }
                Err(_) => {
                    // 否则输出一个异常
                    let request_id = response.headers().get(RQEUST_ID_HEADER);
                    tracing::warn!(
                        "Failed to parse elapsed time: {} for request {:?}",
                        elapsed,
                        request_id
                    );
                }
            }

            Ok(response)
        })
    }
}
