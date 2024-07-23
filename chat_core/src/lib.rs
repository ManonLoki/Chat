pub mod middlewares;
pub mod utils;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::*;
use utoipa::ToSchema;

/// 用户Entity
#[derive(Debug, Clone, ToSchema, FromRow, Serialize, Deserialize, PartialEq)]
pub struct User {
    // 用户ID
    pub id: i64,
    // 工作区ID
    pub ws_id: i64,
    // 用户全名
    pub fullname: String,
    // 用户邮箱
    pub email: String,
    // 密码哈希，序列化时忽略
    #[serde(skip)]
    #[sqlx(default)]
    pub password_hash: Option<String>,
    // 创建时间
    pub created_at: DateTime<Utc>,
}

/// 聊天会话下的用户信息
#[derive(Debug, Clone, ToSchema, FromRow, Serialize, Deserialize, PartialEq)]
pub struct ChatUser {
    // 用户ID
    pub id: i64,
    // 用户全名
    pub fullname: String,
    // 用户邮箱
    pub email: String,
}

/// 工作区Entity
#[derive(Debug, Clone, ToSchema, FromRow, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    /// 工作区ID
    pub id: i64,
    /// 工作区的所有者ID
    pub owner_id: i64,
    /// 工作区名称
    pub name: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 聊天室类型Entity 是一个Scalar
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize, PartialEq, PartialOrd, sqlx::Type)]
#[sqlx(type_name = "chat_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChatType {
    /// 1对1
    Single,
    /// 群组
    Group,
    /// 私有频道
    PrivateChannel,
    /// 公共频道
    PublicChannel,
}

/// 聊天室Entity
#[derive(Debug, Clone, ToSchema, FromRow, Serialize, Deserialize, PartialEq)]
pub struct Chat {
    /// 聊天室Id
    pub id: i64,
    /// 工作空间ID
    pub ws_id: i64,
    /// 聊天室名称
    pub name: Option<String>,
    /// 聊天室类型
    pub r#type: ChatType,
    /// 成员列表  UserIds
    pub members: Vec<i64>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}
/// 消息

#[derive(Debug, Clone, ToSchema, FromRow, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// 消息ID
    pub id: i64,
    /// 聊天室ID
    pub chat_id: i64,
    /// 发送人ID -> User
    pub sender_id: i64,
    /// 聊天内容
    pub content: String,
    /// 文件路径
    pub files: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
impl User {
    pub fn new(id: i64, fullname: &str, email: &str) -> Self {
        Self {
            id,
            ws_id: 0,
            fullname: fullname.to_string(),
            email: email.to_string(),
            password_hash: None,
            created_at: Utc::now(),
        }
    }
}
