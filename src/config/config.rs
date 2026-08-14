use anyhow::{Context, Result};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub log_level: String,
    pub server: ServerConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub recv_buf: usize,
    pub send_buf: usize,
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let config_path = env::var("CONFIG_PATH")
            .with_context(|| "Переменная окружения CONFIG_PATH не задана")?;

        let data = fs::read_to_string(&config_path)
            .with_context(|| format!("Не удалось прочитать конфиг {:?}", config_path))?;

        let cfg: AppConfig = serde_yaml::from_str(&data)
            .with_context(|| format!("Ошибка парсинга конфига {:?}", config_path))?;

        Ok(cfg)
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    pub fn recv_buf(&self) -> Vec<u8> {
        vec![0u8; self.server.recv_buf]
    }

    pub fn log_level(&self) -> LevelFilter {
        match self.log_level.to_lowercase().as_str() {
            "off" => LevelFilter::Off,
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }
}
