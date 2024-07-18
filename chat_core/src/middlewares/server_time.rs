use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::RQEUST_ID_HEADER;
use axum::{extract::Request, response::Response};
use tokio::time::Instant;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct ServerTimeLayer;

impl<S> Layer<S> for ServerTimeLayer {
    type Service = ServerTimeMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ServerTimeMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct ServerTimeMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for ServerTimeMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
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
        let start = Instant::now();
        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response: Response = future.await?;
            let elapsed = format!("{}us", start.elapsed().as_micros());

            match elapsed.parse() {
                Ok(v) => {
                    response.headers_mut().insert("x-server-time", v);
                }
                Err(_) => {
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
