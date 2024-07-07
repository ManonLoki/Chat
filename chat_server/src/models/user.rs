use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use serde::{Deserialize, Serialize};

use crate::{AppError, User};

use super::Workspace;

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

impl User {
    pub async fn find_by_email(email: &str, pool: &sqlx::PgPool) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as(
            r#"SELECT id,ws_id,fullname,email,created_at FROM users WHERE email=$1"#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }
    /// Create a new user
    pub async fn create(input: &CreateUser, pool: &sqlx::PgPool) -> Result<Self, AppError> {
        let user = Self::find_by_email(&input.email, pool).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }

        let ws = match Workspace::find_by_name(&input.workspace, pool).await? {
            Some(ws) => ws,
            None => Workspace::create(&input.workspace, 0, pool).await?,
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
        .fetch_one(pool)
        .await?;

        if ws.owner_id == 0 {
            ws.update_owner(user.id as u64, pool).await?;
        }

        Ok(user)
    }

    pub async fn add_to_workspace(
        &self,
        workspace_id: u64,
        pool: &sqlx::PgPool,
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

    /// Verify email and password
    pub async fn verify(input: &SigninUser, pool: &sqlx::PgPool) -> Result<Option<Self>, AppError> {
        let user: Option<User> = sqlx::query_as(
            r#"SELECT id,ws_id,fullname,email,created_at,password_hash FROM users WHERE email=$1"#,
        )
        .bind(&input.email)
        .fetch_optional(pool)
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
    use std::path::Path;

    use super::*;
    use anyhow::Result;
    use sqlx_db_tester::TestPg;

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
        // Test goes here
        let tdb = TestPg::new(
            "postgres://postgres:880914@localhost:5432".to_string(),
            Path::new("../migrations"),
        );

        let pool = tdb.get_pool().await;

        let input = CreateUser::new("test", "manonloki@gmail.com", "Manon Loki", "loki1988");
        let user = User::create(&input, &pool).await?;

        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = User::find_by_email(&input.email, &pool).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);

        let user = User::verify(&input, &pool).await?;
        assert!(user.is_some());

        Ok(())
    }
}
