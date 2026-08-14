use crate::config::config::AppConfig;
use crate::services::udp_listener::UdpListener;
use anyhow::Result;
use env_logger::Builder;
use std::sync::Arc;
use std::time::Duration;
use log::{error, info};
use tokio::time::interval;
use crate::repositories::ats_users::DatabaseRepository;
use crate::services::user_collector::UserCollector;

pub async fn run(cfg: &AppConfig) -> Result<()> {
    Builder::new().filter_level(cfg.log_level()).init();

    info!("Запуск приложения...");

    let db_repository = Arc::new(
        DatabaseRepository::new(&cfg.db_addr()).await?
    );
    info!("База данных подключена");

    let collector = Arc::new(UserCollector::new(db_repository.clone()));

    let collector_clone = collector.clone();
    tokio::spawn(async move {
        if let Err(e) = collector_clone.start_collecting().await {
            error!("Коллектор остановлен с ошибкой: {}", e);
        }
    });
    info!("Коллектор пользователей запущен");

    let udp_listener = UdpListener::new(cfg.clone(), collector.clone()).await?;
    info!("UDP сервер запущен на {}", cfg.service_addr());

    let stats_collector = collector.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok((total, active)) = stats_collector.get_stats().await {
                info!("Статистика: всего={}, активно={}", total, active);
            }
        }
    });

    udp_listener.start_listening().await?;

    Ok(())
}
