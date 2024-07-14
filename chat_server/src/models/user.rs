use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{AppError, AppState, User};

use super::ChatUser;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    pub fullname: String,
    pub email: String,
    pub password: String,
    pub workspace: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SigninUser {
    pub email: String,
    pub password: String,
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

impl AppState {
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as(
            r#"SELECT id,ws_id,fullname,email,created_at FROM users WHERE email=$1"#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
    /// Create a new user
    pub async fn create_user(&self, input: &CreateUser) -> Result<User, AppError> {
        let user = self.find_user_by_email(&input.email).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }

        let ws = match self.find_workspace_by_name(&input.workspace).await? {
            Some(ws) => ws,
            None => self.create_workspace(&input.workspace, 0).await?,
        };

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

        if ws.owner_id == 0 {
            ws.update_owner(user.id as u64, &self.pool).await?;
        }

        Ok(user)
    }

    /// Verify email and password
    pub async fn verify_user(&self, input: &SigninUser) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            r#"SELECT id,ws_id,fullname,email,created_at,password_hash FROM users WHERE email=$1"#,
        )
        .bind(&input.email)
        .fetch_optional(&self.pool)
        .await?;

        match user {
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

    #[allow(dead_code)]
    pub async fn find_user_by_id(&self, id: u64) -> Result<Option<User>, AppError> {
        let user =
            sqlx::query_as(r#"SELECT id,ws_id,fullname,email,created_at FROM users WHERE id=$1"#)
                .bind(id as i64)
                .fetch_optional(&self.pool)
                .await?;

        Ok(user)
    }
}

impl User {
    pub async fn add_to_workspace(
        &self,
        workspace_id: u64,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        let user = sqlx::query_as(
            r#"
                UPDATE users SET ws_id=$1 WHERE id=$2 AND ws_id=0
                RETURNING id,ws_id,fullname,email,created_at
            "#,
        )
        .bind(self.id)
        .bind(workspace_id as i64)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }
}

impl AppState {
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

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(password_hash)?;

    let is_valid = argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();

    Ok(is_valid)
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
