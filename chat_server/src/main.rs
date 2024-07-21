use anyhow::Result;
use chat_server::{get_router, AppConfig, AppState};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::{format::FmtSpan, Layer},
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
    Layer as _,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let console_layer = Layer::new()
        .with_span_events(FmtSpan::CLOSE)
        .pretty()
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry().with(console_layer).init();

    let config = AppConfig::load()?;
    let port = config.server.port;

    let addr = format!("0.0.0.0:{}", port);

    let state = AppState::try_new(config).await?;

    let app = get_router(state).await?;

    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
