use anyhow::{Context, Result};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;
use std::{env, fmt};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DriverType {
    Postgres,
}

impl fmt::Display for DriverType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverType::Postgres => write!(f, "postgres"),
        }
    }
}

impl FromStr for DriverType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "postgres" => Ok(DriverType::Postgres),
            _ => Err(format!("Неизвестный driver: {}", s)),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub log_level: String,
    pub db: DbConfig,
    pub server: ServerConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub driver: DriverType,
    pub database: String,
    pub host: String,
    pub port: u16,
    pub credentials: CredentialsConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CredentialsConfig {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub recv_buf: usize,
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

    pub fn service_addr(&self) -> String {
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

    pub fn db_addr(&self) -> String {
        format!(
            "{}://{}:{}@{}:{}/{}",
            self.db.driver,
            self.db.credentials.username,
            self.db.credentials.password,
            self.db.host,
            self.db.port,
            self.db.database,
        )
    }
}
