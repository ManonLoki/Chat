use crate::AppError;

use super::{ChatUser, Workspace};

impl Workspace {
    /// Create a new workspace
    pub async fn create(name: &str, owner_id: u64, pool: &sqlx::PgPool) -> Result<Self, AppError> {
        let ws = sqlx::query_as(
            r#"
                INSERT INTO workspaces (name,owner_id)
                VALUES ($1,$2)
                RETURNING id,owner_id,name,created_at
            "#,
        )
        .bind(name)
        .bind(owner_id as i64)
        .fetch_one(pool)
        .await?;
        Ok(ws)
    }

    pub async fn update_owner(&self, owner_id: u64, pool: &sqlx::PgPool) -> Result<Self, AppError> {
        let ws = sqlx::query_as(
            r#"
                UPDATE workspaces SET owner_id=$1 WHERE id=$2 AND  (SELECT ws_id from users WHERE id = $1) = $2
                RETURNING id,owner_id,name,created_at
            "#,
        )

        .bind(owner_id as i64)
        .bind(self.id )
        .fetch_one(pool)
        .await?;

        Ok(ws)
    }

    #[allow(dead_code)]
    pub async fn fetch_all_chat_users(
        id: u64,
        pool: &sqlx::PgPool,
    ) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
                SELECT id,fullname,email
                FROM users
                WHERE ws_id=$1 ORDER BY id ASC
            "#,
        )
        .bind(id as i64)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    pub async fn find_by_name(name: &str, pool: &sqlx::PgPool) -> Result<Option<Self>, AppError> {
        let ws =
            sqlx::query_as(r#"SELECT id,owner_id,name,created_at FROM workspaces WHERE name=$1"#)
                .bind(name)
                .fetch_optional(pool)
                .await?;

        Ok(ws)
    }

    #[allow(dead_code)]
    pub async fn find_by_id(id: u64, pool: &sqlx::PgPool) -> Result<Option<Self>, AppError> {
        let ws =
            sqlx::query_as(r#"SELECT id,owner_id,name,created_at FROM workspaces WHERE id=$1"#)
                .bind(id as i64)
                .fetch_optional(pool)
                .await?;

        Ok(ws)
    }
}

#[cfg(test)]
mod tests {
    use crate::{models::CreateUser, AppConfig, AppState, User};

    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn workspace_should_create() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;

        let pool = &state.pool;

        let ws = Workspace::create("test", 0, pool).await?;

        let input = CreateUser::new(&ws.name, "manonloki", "manonloki@gmail.com", "test");
        let user = User::create(&input, pool).await?;

        let ws = ws.update_owner(user.id as u64, pool).await?;
        assert_eq!(ws.name, "test");
        assert_eq!(ws.id, user.ws_id);
        assert_eq!(ws.owner_id, user.id);

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;

        let pool = &state.pool;

        let ws = Workspace::create("test", 0, pool).await?;

        let ws = Workspace::find_by_name(&ws.name, pool).await?;
        assert!(ws.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;

        let pool = &state.pool;

        let ws = Workspace::create("test", 0, pool).await?;
        let input = CreateUser::new(&ws.name, "Manon Loki", "manonloki@gmail.com", "test");
        let user1 = User::create(&input, pool).await?;
        let input = CreateUser::new(&ws.name, "Alice", "alice@gmail.com", "test");
        let user2 = User::create(&input, pool).await?;

        let users = Workspace::fetch_all_chat_users(ws.id as _, pool).await?;
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].fullname, user1.fullname);
        assert_eq!(users[1].email, user2.email);

        Ok(())
    }
}
