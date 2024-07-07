use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{
    models::{CreateUser, SigninUser},
    AppError, AppState, User,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthOutput {
    token: String,
}

pub(crate) async fn signup_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::create(&input, &state.pool).await?;
    let token = state.ek.sign(user)?;
    // let mut header = HeaderMap::new();
    // header.insert("X-Token", token.parse()?);
    // Ok((StatusCode::CREATED, header))

    let output = AuthOutput { token };

    Ok((StatusCode::CREATED, Json(output)).into_response())
}

pub(crate) async fn signin_handler(
    State(state): State<AppState>,
    Json(input): Json<SigninUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::verify(&input, &state.pool).await?;
    match user {
        Some(user) => {
            let token = state.ek.sign(user)?;
            let output = AuthOutput { token };

            Ok((StatusCode::OK, Json(output)).into_response())
        }
        None => Ok((StatusCode::FORBIDDEN, "Invalid email or password").into_response()),
    }
}

#[cfg(test)]
mod tests {
    use crate::AppConfig;

    use super::*;
    use anyhow::Result;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn signin_up_should_work() -> Result<()> {
        let config = AppConfig::load()?;

        let (_tdb, state) = AppState::new_for_test(config).await?;

        let input = CreateUser::new("test", "Manonloki", "manonloki@gmail.com", "loki1988");
        let response = signup_handler(State(state), Json(input))
            .await?
            .into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body();
        let bytes = body.collect().await?.to_bytes();
        let output: AuthOutput = serde_json::from_slice(&bytes)?;
        assert_ne!(output.token, "");
        Ok(())
    }

    #[tokio::test]
    async fn signin_should_work() -> Result<()> {
        let config = AppConfig::load()?;

        let (_tdb, state) = AppState::new_for_test(config).await?;

        let input = CreateUser::new("test", "Alice", "alice@acme.org", "alice1988");
        User::create(&input, &state.pool).await?;

        let input = SigninUser::new("alice@acme.org", "alice1988");
        let response = signin_handler(State(state), Json(input))
            .await?
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body();
        let bytes = body.collect().await?.to_bytes();
        let output: AuthOutput = serde_json::from_slice(&bytes)?;
        assert_ne!(output.token, "");

        Ok(())
    }
}
