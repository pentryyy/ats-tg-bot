use crate::config::config::AppConfig;
use crate::services::udp_listener::UdpListener;
use anyhow::Result;
use env_logger::Builder;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(cfg: &AppConfig) -> Result<()> {
    Builder::new().filter_level(cfg.log_level()).init();

    let chat_ids = Arc::new(Mutex::new(Vec::new()));
    let cfg_clone = cfg.clone();

    let listener = UdpListener::new(cfg_clone, chat_ids).await?;
    println!("UDP сервер запущен на {}", cfg.addr());

    listener.start_listening().await?;

    Ok(())
}
