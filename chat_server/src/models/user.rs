use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{AppError, AppState};

use chat_core::{ChatUser, User};

/// 注册用户 DTO
#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct CreateUser {
    /// 用户全名
    pub fullname: String,
    /// Email
    pub email: String,
    /// 密码 非Hash
    pub password: String,
    /// 工作空间
    pub workspace: String,
}

/// 登录DTO
#[derive(Debug, ToSchema, Serialize, Deserialize)]
pub struct SigninUser {
    /// Email
    pub email: String,
    /// 密码 非Hash
    pub password: String,
}

/// 实现AppState上下文
impl AppState {
    /// 通过Email查找用户
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as(
            r#"SELECT id,ws_id,fullname,email,created_at FROM users WHERE email=$1"#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
    ///  注册一个新用户
    pub async fn create_user(&self, input: &CreateUser) -> Result<User, AppError> {
        // 先检查用户是否已存在
        let user = self.find_user_by_email(&input.email).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }

        // 再检查workspace是否已存在
        let ws = match self.find_workspace_by_name(&input.workspace).await? {
            Some(ws) => ws,
            None => self.create_workspace(&input.workspace, 0).await?,
        };

        // 使用Argon2加密密码
        let password_hash = hash_password(&input.password)?;
        let user: User = sqlx::query_as(
            r#"
                INSERT INTO users (ws_id,email,fullname,password_hash)
                VALUES ($1,$2,$3,$4)
                RETURNING id,ws_id,fullname,email,created_at
            "#,
        )
        .bind(ws.id)
        .bind(&input.email)
        .bind(&input.fullname)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        // 如果是新建的workspace 则为workspace设置owner
        if ws.owner_id == 0 {
            self.update_workspace_owner(ws.id as u64, user.id as u64)
                .await?;
        }

        Ok(user)
    }

    /// 登录验证
    pub async fn verify_user(&self, input: &SigninUser) -> Result<Option<User>, AppError> {
        // 先查找用户
        let user: Option<User> = sqlx::query_as(
            r#"SELECT id,ws_id,fullname,email,created_at,password_hash FROM users WHERE email=$1"#,
        )
        .bind(&input.email)
        .fetch_optional(&self.pool)
        .await?;

        match user {
            // 存在用户则验证密码
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let is_valid =
                    verify_password(&input.password, &password_hash.unwrap_or_default())?;
                if is_valid {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// 通过ID查找用户
    pub async fn find_user_by_id(&self, id: u64) -> Result<Option<User>, AppError> {
        let user =
            sqlx::query_as(r#"SELECT id,ws_id,fullname,email,created_at FROM users WHERE id=$1"#)
                .bind(id as i64)
                .fetch_optional(&self.pool)
                .await?;

        Ok(user)
    }

    // 根据ws_id获取用户列表
    pub async fn fetch_chat_users(&self, ws_id: u64) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
                SELECT id,fullname,email
                FROM users
                WHERE ws_id=$1 ORDER BY id ASC
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    // 根据user_id获取用户列表
    pub async fn fetch_chat_user_by_ids(&self, id: &[i64]) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
                SELECT id,fullname,email
                FROM users
                WHERE id = ANY($1) ORDER BY id ASC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

// 将密码转为Argon2 hash
fn hash_password(password: &str) -> Result<String, AppError> {
    // 生成随机Salt
    let salt = SaltString::generate(&mut OsRng);
    // 使用Argon2加密
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

// 验证密码
fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(password_hash)?;
    let is_valid = argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();

    Ok(is_valid)
}

#[cfg(test)]
impl CreateUser {
    pub fn new(workspace: &str, fullname: &str, email: &str, password: &str) -> Self {
        Self {
            fullname: fullname.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            workspace: workspace.to_string(),
        }
    }
}

#[cfg(test)]
impl SigninUser {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;

    #[test]
    fn hash_password_and_verify_should_work() -> Result<()> {
        let password = "loki1988";
        let password_hash = hash_password(password)?;
        assert_eq!(password_hash.len(), 97);
        assert!(verify_password(password, &password_hash)?);
        Ok(())
    }

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = CreateUser::new("test", "manonloki2@gmail.com", "Manon Loki2", "loki1988");
        let user = state.create_user(&input).await?;

        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = state.find_user_by_email(&input.email).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);

        let user = state.verify_user(&input).await?;
        assert!(user.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn find_user_by_id_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let user = state.find_user_by_id(1).await?;
        assert!(user.is_some());
        Ok(())
    }
}
