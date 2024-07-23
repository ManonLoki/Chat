use std::{env, fs::File};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// 服务端配置
    pub server: ServerConfig,
    /// 鉴权配置
    pub auth: AuthConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // 优先级：当前目录下的 notify.yaml > /etc/config/notify.yaml > 环境变量 NOTIFY_CONFIG 指定的文件
        let ret = match (
            File::open("notify.yaml"),
            File::open("/etc/config/notify.yaml"),
            env::var("NOTIFY_CONFIG"),
        ) {
            (Ok(file), _, _) => serde_yaml::from_reader(file),
            (_, Ok(file), _) => serde_yaml::from_reader(file),
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?),
            _ => anyhow::bail!("No config file found"),
        };

        Ok(ret?)
    }
}

/// 服务端配置
#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    /// 端口
    pub port: u16,
    /// 数据库链接字符串
    pub db_url: String,
}

/// 鉴权配置
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthConfig {
    /// JWT 公钥
    pub pk: String,
}
