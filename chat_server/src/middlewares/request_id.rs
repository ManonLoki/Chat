use axum::{extract::Request, middleware::Next, response::Response};

use super::RQEUST_ID_HEADER;

pub async fn set_request_id(mut req: Request, next: Next) -> Response {
    // if x-request-id is already set, do nothing
    let id = match req.headers().get(RQEUST_ID_HEADER) {
        Some(v) => Some(v.to_owned()),
        None => {
            let request_id = uuid::Uuid::now_v7().to_string();

            match request_id.parse() {
                Ok(request_id) => {
                    req.headers_mut().insert(RQEUST_ID_HEADER, request_id);
                    req.headers().get(RQEUST_ID_HEADER).map(|v| v.to_owned())
                }
                Err(_) => {
                    tracing::warn!("failed to parse request id: {}", request_id);
                    None
                }
            }
        }
    };

    let mut res = next.run(req).await;

    let Some(id) = id else {
        return res;
    };

    res.headers_mut().insert(RQEUST_ID_HEADER, id);

    res
}
