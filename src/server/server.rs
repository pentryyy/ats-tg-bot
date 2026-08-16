use crate::config::config::AppConfig;
use crate::repositories::chat_users::DatabaseRepository;
use crate::services::udp_listener::UdpListener;
use crate::services::user_collector::UserCollector;
use crate::traits::user_collector::UserCollectorTrait;
use crate::types::bot_command::AtsBotCommand;
use anyhow::{Context, Result};
use env_logger::Builder;
use log::{error, info};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use teloxide::Bot;
use teloxide::prelude::Message;
use tokio::time::interval;

pub async fn run(cfg: &AppConfig) -> Result<()> {
    Builder::new().filter_level(cfg.log_level()).init();

    info!("Запуск приложения...");

    let bot_token = env::var("TELOXIDE_TOKEN")
        .with_context(|| "Переменная окружения TELOXIDE_TOKEN не задана")?;
    let bot = Bot::new(bot_token);
    info!("TG бот подключен");

    let db_repository = Arc::new(DatabaseRepository::new(&cfg.db_addr()).await?);
    info!("База данных подключена");

    let collector = Arc::new(UserCollector::new(cfg.clone(), db_repository.clone()));

    let collector_clone = collector.clone();
    tokio::spawn(async move {
        if let Err(e) = collector_clone.start_collecting().await {
            error!("Коллектор остановлен с ошибкой: {}", e);
        }
    });
    info!("Коллектор пользователей запущен");

    let udp_listener = UdpListener::new(cfg.clone(), bot.clone(), collector.clone()).await?;
    info!("UDP сервер запущен на {}", cfg.service_addr());
    tokio::spawn(async move {
        if let Err(e) = udp_listener.start_listening().await {
            error!("UDP слушатель остановлен с ошибкой: {}", e);
        }
    });

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

    info!("Telegram бот запущен, ожидаем сообщения...");
    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let collector = collector.clone();
        async move {
            if let Some(text) = msg.text() {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if let Some(cmd_str) = parts.get(0) {
                    let command = AtsBotCommand::from_str(cmd_str);

                    if let Some(cmd) = command {
                        cmd.command_handler(bot, msg, collector).await?;
                    }
                }
            }
            Ok(())
        }
    })
    .await;

    Ok(())
}
