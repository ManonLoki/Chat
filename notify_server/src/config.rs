use std::{env, fs::File};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // Reade from /etc/config/notify.yaml or ./notify.yaml or fro env NOTIFY_CONFIG

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

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthConfig {
    pub pk: String,
}
