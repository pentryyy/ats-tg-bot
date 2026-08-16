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
    init_logger(cfg);
    info!("Запуск приложения...");

    let bot = create_bot()?;
    let db_repo = Arc::new(DatabaseRepository::new(&cfg.db_addr()).await?);
    let collector = Arc::new(UserCollector::new(cfg.clone(), db_repo.clone()));

    spawn_collector(collector.clone());
    spawn_udp_listener(cfg, bot.clone(), collector.clone()).await?;
    spawn_stats_reporter(collector.clone());

    info!("Telegram бот запущен, ожидаем сообщения...");
    teloxide::repl(bot, move |bot, msg| {
        let collector = collector.clone();
        async move { handle_message(bot, msg, collector).await }
    })
    .await;

    Ok(())
}

fn init_logger(cfg: &AppConfig) {
    Builder::new().filter_level(cfg.log_level()).init();
}

fn create_bot() -> Result<Bot> {
    let token = env::var("TELOXIDE_TOKEN")
        .with_context(|| "Переменная окружения TELOXIDE_TOKEN не задана")?;
    Ok(Bot::new(token))
}

fn spawn_collector(collector: Arc<UserCollector>) {
    tokio::spawn(async move {
        if let Err(e) = collector.start_collecting().await {
            error!("Коллектор остановлен с ошибкой: {}", e);
        }
    });
    info!("Коллектор пользователей запущен");
}

async fn spawn_udp_listener(
    cfg: &AppConfig,
    bot: Bot,
    collector: Arc<UserCollector>,
) -> Result<()> {
    let listener = UdpListener::new(cfg.clone(), bot, collector).await?;
    info!("UDP сервер запущен на {}", cfg.service_addr());
    tokio::spawn(async move {
        if let Err(e) = listener.start_listening().await {
            error!("UDP слушатель остановлен с ошибкой: {}", e);
        }
    });
    Ok(())
}

fn spawn_stats_reporter(collector: Arc<UserCollector>) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok((total, active)) = collector.get_stats().await {
                info!("Статистика: всего={}, активно={}", total, active);
            }
        }
    });
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    collector: Arc<UserCollector>,
) -> Result<(), teloxide::RequestError> {
    if let Some(text) = msg.text() {
        if let Some(cmd_str) = text.split_whitespace().next() {
            if let Some(cmd) = AtsBotCommand::from_str(cmd_str) {
                cmd.command_handler(bot, msg, collector).await?;
            }
        }
    }
    Ok(())
}
