mod config;
mod dto;
mod services;

use crate::config::config::AppConfig;
use crate::services::udp_listener::UdpListener;
use anyhow::Result;
use env_logger::Builder;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = AppConfig::load()?;

    Builder::new().filter_level(cfg.log_level()).init();

    let udp_port = cfg.server.port;
    let chat_ids = Arc::new(Mutex::new(Vec::new()));

    tokio::spawn(async move {
        match UdpListener::new(cfg, chat_ids).await {
            Ok(listener) => {
                println!("UDP сервер запущен на порту {}", udp_port);
                if let Err(e) = listener.start_listening().await {
                    eprintln!("Ошибка в UDP сервере: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Не удалось запустить UDP сервер: {}", e);
            }
        }
    });

    Ok(())
}
