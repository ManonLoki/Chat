use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::AppError;

use super::{Chat, ChatType, ChatUser};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateChat {
    pub name: Option<String>,
    pub members: Option<Vec<i64>>,
}

impl Chat {
    pub async fn create(
        input: CreateChat,
        ws_id: u64,
        pool: &sqlx::PgPool,
    ) -> Result<Self, AppError> {
        let len = input.members.len();
        if len < 2 {
            return Err(AppError::CreateChatError(
                "Chat mut have at latest 2 members".to_string(),
            ));
        }

        if len > 8 && input.name.is_none() {
            return Err(AppError::CreateChatError(
                "Group chat with more than 8 members must have a name".to_string(),
            ));
        }

        // verify all members exist
        let users = ChatUser::fetch_by_ids(&input.members, pool).await?;
        if users.len() != len {
            return Err(AppError::CreateChatError(
                "Some members do not exist".to_string(),
            ));
        }

        let chat_type = match (&input.name, len) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_name), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };

        let chat = sqlx::query_as(
            r#"
                INSERT INTO chats (ws_id,type,name,members)
                VALUES ($1,$2,$3,$4)
                RETURNING id,ws_id,name,type,members,created_at
            "#,
        )
        .bind(ws_id as i64)
        .bind(chat_type)
        .bind(input.name)
        .bind(&input.members)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_all(ws_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let chats = sqlx::query_as(
            r#"
                SELECT id,ws_id,name,type,members,created_at
                FROM chats
                WHERE ws_id=$1
                ORDER BY id ASC
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let chat = sqlx::query_as(
            r#"
                SELECT id,ws_id,name,type,members,created_at
                FROM chats
                WHERE id=$1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;

        Ok(chat)
    }

    pub async fn update_by_id(id: u64, input: UpdateChat, pool: &PgPool) -> Result<(), AppError> {
        // 校验参数是否有效
        if input.name.is_none() && input.members.is_none() {
            return Err(AppError::NotChange(
                "At least one field must be provided".to_string(),
            ));
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE chats SET ");
        let mut first_field = true;

        if let Some(name) = input.name {
            if !first_field {
                query_builder.push(",");
            }
            query_builder.push(" name =").push_bind(name);
            first_field = false;
        }

        if let Some(members) = input.members {
            if !first_field {
                query_builder.push(",");
            }
            query_builder.push(" members =").push_bind(members.clone());
        }

        query_builder.push(" WHERE id =").push_bind(id as i64);

        let query = query_builder.build();

        query.execute(pool).await?;

        Ok(())
    }

    pub async fn delete_by_id(id: u64, pool: &PgPool) -> Result<(), AppError> {
        sqlx::query("DELETE FROM chats WHERE id=$1")
            .bind(id as i64)
            .execute(pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
impl CreateChat {
    pub fn new(name: &str, members: &[i64], public: bool) -> Self {
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::get_test_pool;

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateChat::new("", &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");

        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);
    }

    #[tokio::test]
    async fn create_public_named_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateChat::new("general", &[1, 2, 3], true);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");

        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 3);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
    }

    #[tokio::test]
    async fn chat_get_by_id_should_work() -> anyhow::Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let chat = Chat::get_by_id(1, &pool).await?.expect("chat not found");
        assert_eq!(chat.id, 1);
        assert_eq!(chat.name.unwrap(), "general");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 5);

        Ok(())
    }

    #[tokio::test]
    async fn chat_fetch_all_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;

        let chats = Chat::fetch_all(1, &pool).await.expect("chat fetch failed");
        assert_eq!(chats.len(), 4);
    }

    #[tokio::test]
    async fn chat_update_by_id_should_work() -> anyhow::Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = UpdateChat {
            name: Some("new name".to_string()),
            members: Some(vec![1, 2, 3]),
        };

        Chat::update_by_id(1, input, &pool).await?;

        let chat = Chat::get_by_id(1, &pool).await?.expect("chat not found");
        assert_eq!(chat.name.unwrap(), "new name");
        assert_eq!(chat.members.len(), 3);

        Ok(())
    }
}
