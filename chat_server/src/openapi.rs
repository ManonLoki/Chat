use axum::Router;
use chat_core::{Chat, ChatType, ChatUser, Message, User, Workspace};
use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable as _};
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    error::ErrorOutput,
    models::{CreateChat, CreateMessage, CreateUser, ListMessages, SigninUser},
    AppState, AuthOutput,
};

use crate::handlers::*;

/// OpenAPI Router Trait
/// 用于附加OpenAPI文档
pub(crate) trait OpenApiRouter {
    fn openapi(self) -> Self;
}

// 创建AppDoc结构体
#[derive(OpenApi)]
#[openapi(
        paths(
            signup_handler,
            signin_handler,
            list_chat_handler,
            create_chat_handler,
            update_chat_handler,
            delete_chat_handler,
            get_chat_handler,
            delete_chat_handler,
            list_message_handler,
            send_message_handler,
            list_chat_users_handler
        ),
        components(
            schemas(User, Chat, ChatType, ChatUser, Message, Workspace, SigninUser, CreateUser, CreateChat, CreateMessage, ListMessages, AuthOutput, ErrorOutput),
        ),
        modifiers(&SecurityAddon),
    )]
pub(crate) struct ApiDoc;

// 创建鉴权插件
struct SecurityAddon;
// 实现Modify Trait
impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "token",
                SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
            )
        }
    }
}

/// 为axum::Router实现OpenApiRouter
impl OpenApiRouter for Router<AppState> {
    fn openapi(self) -> Self {
        self.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .merge(Redoc::with_url("/redoc", ApiDoc::openapi()))
            .merge(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"))
    }
}
