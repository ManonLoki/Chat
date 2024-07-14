use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};

use tokio::fs;

use tracing::{info, warn};

use crate::{
    models::{ChatFile, CreateMessage, ListMessages},
    AppError, AppState, User,
};

pub(crate) async fn send_message_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<CreateMessage>,
) -> Result<impl IntoResponse, AppError> {
    let msg = state.create_message(input, id, user.id as _).await?;

    Ok(Json(msg))
}

pub(crate) async fn list_message_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(input): Query<ListMessages>,
) -> Result<impl IntoResponse, AppError> {
    let messages = state.list_messages(input, id).await?;

    Ok(Json(messages))
}

pub(crate) async fn upload_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let ws_id = user.ws_id as u64;
    let base_dir = &state.config.server.base_dir;
    let mut files = vec![];
    while let Some(field) = multipart.next_field().await.expect("read error") {
        let filename = field.file_name().map(|name| name.to_string());

        let (Some(filename), Ok(data)) = (filename, field.bytes().await) else {
            warn!("Failed to read multi part field");
            continue;
        };

        let file = ChatFile::new(ws_id, &filename, &data);
        let path = file.path(base_dir);
        if path.exists() {
            info!("file already exists")
        } else {
            tokio::fs::create_dir_all(path.parent().unwrap()).await?;

            tokio::fs::write(path, data).await?;

            files.push(file.url())
        }
    }
    Ok(Json(files))
}

pub(crate) async fn file_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path((ws_id, path)): Path<(u64, String)>,
) -> Result<impl IntoResponse, AppError> {
    if user.ws_id as u64 != ws_id {
        return Err(AppError::NotFound(
            "File doesn't exist or you don't have this file".to_string(),
        ));
    }

    let base_dir = state.config.server.base_dir.join(ws_id.to_string());
    let path = base_dir.join(path);
    if !path.exists() {
        return Err(AppError::NotFound("File Not Found".to_string()));
    }

    let mime = mime_guess::from_path(&path).first_or_octet_stream();

    let body = fs::read(path).await?;

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", mime.to_string().parse().unwrap());

    Ok((headers, body))
}
