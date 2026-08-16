use anyhow::{Context, Result};
use log::LevelFilter;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::time::Duration;
use std::{env, fmt};

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    humantime::parse_duration(&s).map_err(|e| {
        serde::de::Error::custom(format!("некорректное значение result_delay={:?}: {}", s, e))
    })
}

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub log_level: String,
    pub user_collector: UserCollectorConfig,
    pub db: DbConfig,
    pub server: ServerConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserCollectorConfig {
    #[serde(deserialize_with = "deserialize_duration")]
    pub update_interval: Duration,

    #[serde(deserialize_with = "deserialize_duration")]
    pub deactivate_after: Duration,

    #[serde(deserialize_with = "deserialize_duration")]
    pub cleanup_after: Duration,

    #[serde(deserialize_with = "deserialize_duration")]
    pub cleanup_interval: Duration,
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

    pub fn deactivate_after_minutes(&self) -> i64 {
        (self.user_collector.deactivate_after.as_secs() / 60) as i64
    }

    pub fn cleanup_after_days(&self) -> i64 {
        (self.user_collector.cleanup_after.as_secs() / 86400) as i64
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
