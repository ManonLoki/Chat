use std::{env, fs::File, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// 服务器配置
    pub server: ServerConfig,
    /// 认证配置
    pub auth: AuthConfig,
}

impl AppConfig {
    /// 加载配置
    pub fn load() -> Result<Self> {
        // 尝试从当前目录/系统配置目录/环境变量中加载配置文件
        let ret = match (
            File::open("chat.yaml"),
            File::open("/etc/config/chat.yaml"),
            env::var("CHAT_CONFIG"),
        ) {
            (Ok(file), _, _) => serde_yaml::from_reader(file),
            (_, Ok(file), _) => serde_yaml::from_reader(file),
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?),
            _ => anyhow::bail!("No config file found"),
        };

        Ok(ret?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
    pub base_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthConfig {
    pub sk: String,
    pub pk: String,
}
