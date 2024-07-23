use std::{net::SocketAddr, time::Duration};

use chat_core::{Chat, ChatType, Message};
use futures::StreamExt as _;
use reqwest::{
    multipart::{Form, Part},
    Client, StatusCode,
};
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpListener;

const WILD_ADDR: &str = "0.0.0.0:0";

/// 实现Token模型
#[derive(Debug, Deserialize)]
struct AuthOutput {
    token: String,
}
/// ChatServer实例
struct ChatServer {
    /// 监听地址
    addr: SocketAddr,
    /// Token
    token: String,
    /// Reqwest Client
    client: Client,
}

impl ChatServer {
    async fn new(state: chat_server::AppState) -> anyhow::Result<Self> {
        // 监听路由
        let app = chat_server::get_router(state).await?;
        let listener = TcpListener::bind(WILD_ADDR).await?;
        let addr = listener.local_addr()?;
        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        // 创建Reqwest Client
        let client = reqwest::Client::new();

        let mut ret = Self {
            addr,
            client,
            token: "".to_string(),
        };

        // 完成登录操作
        ret.sign().await?;

        Ok(ret)
    }

    /// 登录
    async fn sign(&mut self) -> anyhow::Result<()> {
        let res = self
            .client
            .post(&format!("http://{}/api/signin", self.addr))
            .header("Content-Type", "application/json")
            .body(r#"{"email": "manonloki@gmail.com","password":"loki1988"}"#)
            .send()
            .await?;

        println!("Response: {:?}", res.status());

        assert_eq!(res.status(), 200);
        let ret: AuthOutput = res.json().await?;
        // 更新Token
        self.token = ret.token;
        Ok(())
    }

    /// 创建聊天室
    async fn create_chat(&self) -> anyhow::Result<Chat> {
        let res = self
            .client
            .post(format!("http://{}/api/chats", self.addr))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .body(r#"{"name": "test", "members": [1, 2], "public": false}"#);

        let res = res.send().await?;
        assert_eq!(res.status(), StatusCode::CREATED);
        let chat: Chat = res.json().await?;
        assert_eq!(chat.name.as_ref().unwrap(), "test");
        assert_eq!(chat.members, vec![1, 2]);
        assert_eq!(chat.r#type, ChatType::PrivateChannel);

        Ok(chat)
    }

    /// 创建消息
    async fn create_message(&self, chat_id: u64) -> anyhow::Result<Message> {
        // 上传文件
        let data = include_bytes!("../Cargo.toml");
        let files = Part::bytes(data)
            .file_name("Cargo.toml")
            .mime_str("text/plain")?;
        let form = Form::new().part("file", files);
        let res = self
            .client
            .post(&format!("http://{}/api/upload", self.addr))
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let ret: Vec<String> = res.json().await?;

        // 创建消息
        let body = serde_json::to_string(&json!({
            "content": "hello",
            "files": ret,
        }))?;
        let res = self
            .client
            .post(format!("http://{}/api/chats/{}", self.addr, chat_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .body(body);
        let res = res.send().await?;
        assert_eq!(res.status(), StatusCode::CREATED);
        let message: Message = res.json().await?;
        assert_eq!(message.content, "hello");
        assert_eq!(message.files, ret);
        assert_eq!(message.sender_id, 1);
        assert_eq!(message.chat_id, chat_id as i64);
        Ok(message)
    }
}

/// 通知服务
struct NotifyServer;

impl NotifyServer {
    async fn new(db_url: &str, token: &str) -> anyhow::Result<Self> {
        // 监听端口
        let mut config = notify_server::AppConfig::load()?;
        config.server.db_url = db_url.to_string();
        let app = notify_server::get_router(config).await?;
        let listener = TcpListener::bind(WILD_ADDR).await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service())
                .await
                .unwrap();
        });

        // 连姐EventSource
        let mut es = EventSource::get(format!("http://{}/events?access_token={}", addr, token));
        // 开启一个新的任务监听EventSource Stream
        tokio::spawn(async move {
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => println!("Connection Open!"),
                    Ok(Event::Message(message)) => match message.event.as_str() {
                        "NewChat" => {
                            let chat: Chat = serde_json::from_str(&message.data).unwrap();
                            assert_eq!(chat.name.as_ref().unwrap(), "test");
                            assert_eq!(chat.members, vec![1, 2]);
                            assert_eq!(chat.r#type, ChatType::PrivateChannel);
                        }
                        "NewMessage" => {
                            let msg: Message = serde_json::from_str(&message.data).unwrap();
                            assert_eq!(msg.content, "hello");
                            assert_eq!(msg.files.len(), 1);
                            assert_eq!(msg.sender_id, 1);
                        }
                        _ => {
                            panic!("Unknown event:{}", message.event);
                        }
                    },
                    Err(err) => {
                        println!("Error:{}", err);
                        es.close();
                    }
                }
            }
        });

        Ok(Self)
    }
}

#[tokio::test]
async fn chat_server_should_work() -> anyhow::Result<()> {
    // 初始化ChatServer
    let (tdb, state) = chat_server::AppState::new_for_test().await?;
    let chat_server = ChatServer::new(state).await?;
    let db_url = tdb.url();

    // 初始化NotifyServer
    NotifyServer::new(&db_url, &chat_server.token).await?;
    // 调用chat_server接口 等待通知
    let chat = chat_server.create_chat().await?;
    let _msg = chat_server.create_message(chat.id as u64).await?;
    // await 等待其他动作完成
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}
