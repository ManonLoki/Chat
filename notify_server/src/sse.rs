use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::{sse::Event, Sse},
    Extension,
};

use chat_core::User;
use futures::Stream;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::warn;

use crate::{AppEvent, AppState};

const CHANNEL_CAPACITY: usize = 100;

/// 处理SSE请求
pub(crate) async fn sse_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // 获取当前登录的User
    let user_id = user.id as u64;

    // 拿到全部的Users
    let users = &state.users;

    let cloned_uesrs = users.clone();
    // 获取用户的Receiver 这里传递的是AppEvent
    let rx = if let Some(tx) = users.get(&user_id) {
        tx.subscribe()
    } else {
        let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
        users.insert(user_id, tx);
        rx
    };
    // 创建SSE Stream
    let stream = BroadcastStream::new(rx)
        .filter_map(move |v| {
            v.inspect_err(|e| {
                warn!("Failed to send event: {}", e);
                cloned_uesrs.remove(&user_id);
            })
            .ok()
        })
        .map(|v| {
            let name = match v.as_ref() {
                AppEvent::NewChat(_) => "NewChat",
                AppEvent::AddToChat(_) => "AddToChat",
                AppEvent::RemoveFromChat(_) => "RemoveFromChat",
                AppEvent::NewMessage(_) => "NewMessage",
            };

            let v = serde_json::to_string(&v).expect("Failed to serialize event");
            Ok(Event::default().data(v).event(name))
        });
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
