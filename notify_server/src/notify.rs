use std::{collections::HashSet, sync::Arc};

use chat_core::{Chat, Message};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgListener;

use tracing::{info, warn};

use crate::AppState;

/// 应用事件
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    /// 创建新聊天
    NewChat(Chat),
    /// 加入聊天室
    AddToChat(Chat),
    /// 离开聊天室
    RemoveFromChat(Chat),
    /// 新消息
    NewMessage(Message),
}

/// 通知数据结构
#[derive(Debug)]
struct Notification {
    /// 用户ID列表
    user_ids: HashSet<u64>,
    /// 事件
    event: Arc<AppEvent>,
}

/// ChatUpdated 通知结构
#[derive(Debug, Serialize, Deserialize)]
struct ChatUpdated {
    /// 操作
    op: String,
    /// 旧数据 在Update和Delete时肯定存在
    old: Option<Chat>,
    /// 新数据 在Insert和Update时肯定存在
    new: Option<Chat>,
}

/// ChatMessageCreated 通知结构
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessageCreated {
    /// 消息数据
    message: Message,
    /// 成员列表
    members: Vec<i64>,
}

/// 设置PgListener
pub async fn setup_pg_listener(state: AppState) -> anyhow::Result<()> {
    // 连接PgListener
    let mut listener = PgListener::connect(&state.config.server.db_url).await?;
    // 监听触发器
    listener.listen("chat_updated").await?;
    listener.listen("chat_message_created").await?;

    // 转换为Stream
    let mut stream = listener.into_stream();

    // 开启一个新的任务监听
    tokio::spawn(async move {
        while let Some(Ok(notif)) = stream.next().await {
            info!("Received notification: {:?}", notif);
            // 加载通知
            let notification = Notification::load(notif.channel(), notif.payload())?;
            // 获取当前连接进来的用户
            let users = &state.users;
            // 遍历需要通知的用户列表
            for user_id in notification.user_ids {
                if let Some(tx) = users.get(&user_id) {
                    // 发送Send
                    if let Err(e) = tx.send(notification.event.clone()) {
                        warn!("Failed to send event to user {}: {}", user_id, e);
                    }
                }
            }
        }

        Ok::<_, anyhow::Error>(())
    });
    Ok(())
}

/// 通知结构实现
impl Notification {
    /// 从PgListener通知中加载
    fn load(r#type: &str, payload: &str) -> anyhow::Result<Self> {
        // 判断触发器的类型
        match r#type {
            "chat_updated" => {
                // 解析ChatUpdated
                let payload: ChatUpdated = serde_json::from_str(payload)?;
                // 获取受影响的用户ID列表 将新老用户做Diff，获取受影响的用户列表
                let user_ids =
                    get_affected_chat_user_ids(payload.old.as_ref(), payload.new.as_ref());
                // 根据操作类型返回不同的事件
                let event = match payload.op.as_str() {
                    "INSERT" => AppEvent::NewChat(payload.new.unwrap()),
                    "UPDATE" => AppEvent::AddToChat(payload.new.unwrap()),
                    "DELETE" => AppEvent::RemoveFromChat(payload.old.unwrap()),
                    _ => return Err(anyhow::anyhow!("Unknown operation: {}", payload.op)),
                };
                // 返回通知
                Ok(Self {
                    user_ids,
                    event: Arc::new(event),
                })
            }
            "chat_message_created" => {
                // 解析ChatMessageCreated
                let payload: ChatMessageCreated = serde_json::from_str(payload)?;
                // 这个就是在Chat里所有的用户
                let user_ids = payload.members.iter().map(|v| *v as u64).collect();

                // 返回通知
                Ok(Self {
                    user_ids,
                    event: Arc::new(AppEvent::NewMessage(payload.message)),
                })
            }
            _ => Err(anyhow::anyhow!("Unknown notification type: {}", r#type)),
        }
    }
}

/// 计算受影响用户
fn get_affected_chat_user_ids(old: Option<&Chat>, new: Option<&Chat>) -> HashSet<u64> {
    match (old, new) {
        // 如果是更新操作 则取交集
        (Some(old), Some(new)) => {
            let old_user_ids: HashSet<_> = old.members.iter().map(|v| *v as u64).collect();
            let new_user_ids: HashSet<_> = new.members.iter().map(|v| *v as u64).collect();
            // 如果没变化 则不需要通知 否则取交集
            if old_user_ids == new_user_ids {
                HashSet::new()
            } else {
                old_user_ids.union(&new_user_ids).copied().collect()
            }
        }
        // 如果是删除操作 则取旧的所有用户
        (Some(old), None) => old.members.iter().map(|v: &i64| *v as u64).collect(),
        // 如果是新增操作 则取新的所有用户
        (None, Some(new)) => new.members.iter().map(|v| *v as u64).collect(),
        _ => HashSet::new(),
    }
}
