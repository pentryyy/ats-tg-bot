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
use teloxide::types::Update;
use teloxide::{
    Bot,
    dispatching::{Dispatcher, UpdateFilterExt},
};
use tokio::signal;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

pub async fn run(cfg: &AppConfig) -> Result<()> {
    init_logger(cfg);
    info!("Запуск приложения...");

    let bot = create_bot()?;
    let db_repo = Arc::new(DatabaseRepository::new(&cfg.db_addr()).await?);
    let collector = Arc::new(UserCollector::new(cfg.clone(), db_repo.clone()));

    let cancel_token = CancellationToken::new();

    spawn_collector(collector.clone(), cancel_token.clone());
    spawn_udp_listener(cfg, bot.clone(), collector.clone(), cancel_token.clone()).await?;
    spawn_stats_reporter(collector.clone(), cancel_token.clone());

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
    let handle = tokio::spawn(async move {
        dispatcher.dispatch().await;
    });

    info!("Telegram бот запущен, ожидаем сообщения...");

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Получен сигнал остановки, начинаем graceful shutdown...");
        }
    }

    if let Err(e) = shutdown_token.shutdown() {
        error!("Ошибка при остановке диспетчера: {}", e);
    }

    cancel_token.cancel();

    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    info!("Приложение завершено");
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

fn spawn_collector(collector: Arc<UserCollector>, cancel_token: CancellationToken) {
    tokio::spawn(async move {
        if let Err(e) = collector.start_collecting(cancel_token).await {
            error!("Коллектор остановлен с ошибкой: {}", e);
        }
    });
    info!("Коллектор пользователей запущен");
}

async fn spawn_udp_listener(
    cfg: &AppConfig,
    bot: Bot,
    collector: Arc<UserCollector>,
    cancel_token: CancellationToken,
) -> Result<()> {
    let socket_service = SocketService::bind(cfg.service_addr()).await?;
    let listener =
        UdpListener::new(cfg.clone(), socket_service, bot, collector, cancel_token).await;
    info!("UDP сервер запущен на {}", cfg.service_addr());
    tokio::spawn(async move {
        if let Err(e) = listener.start_listening().await {
            error!("UDP слушатель остановлен с ошибкой: {}", e);
        }
    });
    Ok(())
}

fn spawn_stats_reporter(collector: Arc<UserCollector>, cancel_token: CancellationToken) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Ok((total, active)) = collector.get_stats().await {
                        info!("Статистика: всего={}, активно={}", total, active);
                    }
                }
                _ = cancel_token.cancelled() => {
                    info!("Репортёр статистики завершает работу");
                    break;
                }
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
