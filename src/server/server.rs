use crate::config::config::AppConfig;
use crate::repositories::chat_users::DatabaseRepository;
use crate::services::socket::SocketService;
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
use teloxide::dptree::{self, deps};
use teloxide::prelude::*;
use teloxide::{
    Bot,
    dispatching::{Dispatcher, UpdateFilterExt},
};
use tokio::signal;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

pub async fn run(cfg: &AppConfig) -> Result<()> {
    init_logger(cfg);
    info!("Запуск сервера...");

    let bot = create_bot()?;
    let db_repo = Arc::new(DatabaseRepository::new(&cfg.db_addr()).await?);
    let collector = Arc::new(UserCollector::new(cfg.clone(), db_repo.clone()));

    let collector_handle = spawn_collector(collector.clone());
    let udp_handle = spawn_udp_listener(cfg, bot.clone(), collector.clone()).await?;
    let stats_handle = spawn_stats_reporter(collector.clone());

    let message_handler = |bot: Bot, msg: Message, collector: Arc<UserCollector>| async move {
        handle_message(bot, msg, collector).await
    };

    let mut dispatcher = Dispatcher::builder(
        bot,
        dptree::entry().branch(Update::filter_message().endpoint(message_handler)),
    )
    .dependencies(deps![collector.clone()])
    .build();

    let shutdown_token = dispatcher.shutdown_token();
    let dispatch_handle = tokio::spawn(async move {
        dispatcher.dispatch().await;
    });

    info!("Telegram бот запущен, ожидаем сообщения...");

    let ctrl_c = signal::ctrl_c();
    tokio::select! {
        _ = ctrl_c => {
            info!("Получен сигнал завершения, останавливаем сервер...");
        }
    }

    if let Err(e) = shutdown_token.shutdown() {
        error!("Ошибка при остановке диспетчера: {}", e);
    }

    collector_handle.abort();
    udp_handle.abort();
    stats_handle.abort();
    dispatch_handle.abort();

    tokio::time::sleep(Duration::from_secs(1)).await;

    info!("Сервер остановлен");
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

fn spawn_collector(collector: Arc<UserCollector>) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
        if let Err(e) = collector.start_collecting().await {
            error!("Коллектор остановлен с ошибкой: {}", e);
        }
    });
    info!("Коллектор пользователей запущен");
    handle
}

async fn spawn_udp_listener(
    cfg: &AppConfig,
    bot: Bot,
    collector: Arc<UserCollector>,
) -> Result<tokio::task::JoinHandle<()>> {
    let socket_service = SocketService::bind(cfg.service_addr()).await?;
    let listener = UdpListener::new(cfg.clone(), socket_service, bot, collector).await;
    let cancel_token = CancellationToken::new();
    info!("UDP сервер запущен на {}", cfg.service_addr());
    let handle = tokio::spawn(async move {
        if let Err(e) = listener.start_listening(cancel_token).await {
            error!("UDP слушатель остановлен с ошибкой: {}", e);
        }
    });
    Ok(handle)
}

fn spawn_stats_reporter(collector: Arc<UserCollector>) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok((total, active)) = collector.get_stats().await {
                info!("Статистика: всего={}, активно={}", total, active);
            }
        }
    });
    info!("Репортер статистики запущен");
    handle
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
