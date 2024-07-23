use crate::{AppError, AppState};

use chat_core::Workspace;

///实现AppState上下文
impl AppState {
    /// 创建Workspace
    pub async fn create_workspace(&self, name: &str, owner_id: u64) -> Result<Workspace, AppError> {
        let ws = sqlx::query_as(
            r#"
                INSERT INTO workspaces (name,owner_id)
                VALUES ($1,$2)
                RETURNING id,owner_id,name,created_at
            "#,
        )
        .bind(name)
        .bind(owner_id as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(ws)
    }

    /// 根据工作空间名称查询Workspace
    pub async fn find_workspace_by_name(&self, name: &str) -> Result<Option<Workspace>, AppError> {
        let ws =
            sqlx::query_as(r#"SELECT id,owner_id,name,created_at FROM workspaces WHERE name=$1"#)
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        Ok(ws)
    }

    /// 根据ID查询Workspace
    pub async fn find_workspace_by_id(&self, id: u64) -> Result<Option<Workspace>, AppError> {
        let ws =
            sqlx::query_as(r#"SELECT id,owner_id,name,created_at FROM workspaces WHERE id=$1"#)
                .bind(id as i64)
                .fetch_optional(&self.pool)
                .await?;

        Ok(ws)
    }
    /// 更新Workspace的所有者
    pub async fn update_workspace_owner(
        &self,
        id: u64,
        owner_id: u64,
    ) -> Result<Workspace, AppError> {
        let ws = sqlx::query_as(
            r#"
                UPDATE workspaces SET owner_id=$1 WHERE id=$2 AND  (SELECT ws_id from users WHERE id = $1) = $2
                RETURNING id,owner_id,name,created_at
            "#,
        )

        .bind(owner_id as i64)
        .bind(id as i64 )
        .fetch_one(&self.pool)
        .await?;

        Ok(ws)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::CreateUser;

    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn workspace_should_create_and_set_owner() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let ws = state.create_workspace("test", 0).await?;

        let input = CreateUser::new(&ws.name, "manonloki1", "manonloki1@gmail.com", "test");
        let user = state.create_user(&input).await?;

        let ws = state
            .update_workspace_owner(ws.id as _, user.id as _)
            .await?;
        assert_eq!(ws.name, "test");
        assert_eq!(ws.id, user.ws_id);
        assert_eq!(ws.owner_id, user.id);

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let ws = state.find_workspace_by_name("acme").await?;
        assert!(ws.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let users = state.fetch_chat_users(1).await?;
        assert_eq!(users.len(), 5);
        assert_eq!(users[0].fullname, "manonloki");
        assert_eq!(users[1].email, "heather@163.com");

        Ok(())
    }
}
