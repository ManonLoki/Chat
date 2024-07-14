use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{models::ChatFile, AppError, AppState};

use super::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    pub files: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMessages {
    pub last_id: Option<u64>,
    pub limit: u64,
}

impl AppState {
    pub async fn create_message(
        &self,
        input: CreateMessage,
        chat_id: u64,
        user_id: u64,
    ) -> Result<Message, AppError> {
        //verify conentent  - not empty
        if input.content.is_empty() {
            return Err(AppError::CreateMessageError("content is empty".to_string()));
        }
        //verify files exist
        for s in &input.files {
            let file = ChatFile::from_str(s)?;
            if !file.path(&self.config.server.base_dir).exists() {
                return Err(AppError::CreateMessageError(
                    "file does not exist".to_string(),
                ));
            }
        }

        // create message
        let message: Message = sqlx::query_as(
            r"
                INSERT INTO messages (chat_id, sender_id, content, files)
                VALUES ($1, $2, $3, $4)
                RETURNING  id,chat_id,sender_id,content,files,created_at
            ",
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .bind(input.content)
        .bind(input.files)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    pub async fn list_messages(
        &self,
        input: ListMessages,
        chat_id: u64,
    ) -> Result<Vec<Message>, AppError> {
        let last_id = input.last_id.unwrap_or(i64::MAX as _);
        let messages: Vec<Message> = sqlx::query_as(
            r#"
                SELECT id,chat_id,sender_id,content,files,created_at
                FROM messages
                WHERE chat_id = $1
                AND id < $2
                ORDER BY id DESC
                LIMIT $3
            "#,
        )
        .bind(chat_id as i64)
        .bind(last_id as i64)
        .bind(input.limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn create_message_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateMessage {
            content: "hello".to_string(),
            files: vec![],
        };

        let message = state
            .create_message(input, 1, 1)
            .await
            .expect("create message failed");

        assert_eq!(message.content, "hello");

        let input = CreateMessage {
            content: "hello".to_string(),
            files: vec!["1".to_string()],
        };

        let err = state.create_message(input, 1, 1).await.unwrap_err();
        assert_eq!(err.to_string(), "invalid chat file path:1");

        let url = upload_dummy_file(&state)?;
        let input = CreateMessage {
            content: "hello".to_string(),
            files: vec![url],
        };

        let message = state.create_message(input, 1, 1).await?;

        assert_eq!(message.content, "hello");
        assert_eq!(message.files.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn list_messages_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = ListMessages {
            last_id: None,
            limit: 6,
        };

        let messages = state.list_messages(input, 1).await?;
        assert_eq!(messages.len(), 6);

        let last_id = messages.last().expect("last message should exists").id;
        let input = ListMessages {
            last_id: Some(last_id as _),
            limit: 6,
        };

        let message = state.list_messages(input, 1).await?;
        assert_eq!(message.len(), 4);

        Ok(())
    }

    fn upload_dummy_file(state: &AppState) -> Result<String> {
        let file = ChatFile::new(1, "test.txt", b"hello world");
        let path = file.path(&state.config.server.base_dir);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, b"hello world")?;
        Ok(file.url())
    }
}