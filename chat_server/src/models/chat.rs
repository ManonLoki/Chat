use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder};
use utoipa::ToSchema;

use crate::{AppError, AppState};

use chat_core::{Chat, ChatType};

#[derive(Debug, Clone, ToSchema, Default, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[derive(Debug, Clone, ToSchema, Default, Serialize, Deserialize)]
pub struct UpdateChat {
    pub name: Option<String>,
    pub members: Option<Vec<i64>>,
}

impl AppState {
    pub async fn create_chat(&self, input: CreateChat, ws_id: u64) -> Result<Chat, AppError> {
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
        let users = self.fetch_chat_user_by_ids(&input.members).await?;
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
        .fetch_one(&self.pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_all_chat(&self, ws_id: u64) -> Result<Vec<Chat>, AppError> {
        let chats = sqlx::query_as(
            r#"
                SELECT id,ws_id,name,type,members,created_at
                FROM chats
                WHERE ws_id=$1
                ORDER BY id ASC
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_chat_by_id(&self, id: u64) -> Result<Option<Chat>, AppError> {
        let chat = sqlx::query_as(
            r#"
                SELECT id,ws_id,name,type,members,created_at
                FROM chats
                WHERE id=$1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(chat)
    }

    pub async fn update_chat_by_id(&self, id: u64, input: UpdateChat) -> Result<(), AppError> {
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

        let query: sqlx::query::Query<Postgres, sqlx::postgres::PgArguments> =
            query_builder.build();

        query.execute(&self.pool).await?;

        Ok(())
    }

    pub async fn delete_chat_by_id(&self, id: u64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM chats WHERE id=$1")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn is_chat_member(&self, chat_id: u64, user_id: u64) -> Result<bool, AppError> {
        let row = sqlx::query(
            r#"
                SELECT 1
                FROM chats
                WHERE id=$1 AND $2 = ANY(members)
            "#,
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
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

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = CreateChat::new("", &[1, 2], false);
        let chat = state
            .create_chat(input, 1)
            .await
            .expect("create chat failed");

        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);

        Ok(())
    }

    #[tokio::test]
    async fn create_public_named_chat_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = CreateChat::new("general", &[1, 2, 3], true);
        let chat = state
            .create_chat(input, 1)
            .await
            .expect("create chat failed");

        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 3);
        assert_eq!(chat.r#type, ChatType::PublicChannel);

        Ok(())
    }

    #[tokio::test]
    async fn chat_get_by_id_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let chat = state.get_chat_by_id(1).await?.expect("chat not found");
        assert_eq!(chat.id, 1);
        assert_eq!(chat.name.unwrap(), "general");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 5);

        Ok(())
    }

    #[tokio::test]
    async fn chat_fetch_all_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let chats = state.fetch_all_chat(1).await.expect("chat fetch failed");
        assert_eq!(chats.len(), 4);
        Ok(())
    }

    #[tokio::test]
    async fn chat_update_by_id_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = UpdateChat {
            name: Some("new name".to_string()),
            members: Some(vec![1, 2, 3]),
        };

        state.update_chat_by_id(1, input).await?;

        let chat = state.get_chat_by_id(1).await?.expect("chat not found");
        assert_eq!(chat.name.unwrap(), "new name");
        assert_eq!(chat.members.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn chat_is_member_should_work() -> anyhow::Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let is_member = state.is_chat_member(1, 1).await?;
        assert!(is_member);

        let is_member = state.is_chat_member(1, 6).await?;
        assert!(!is_member);

        Ok(())
    }
}
