mod config;
mod dto;
mod services;

use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;
use teloxide::{Bot, prelude::*};
use tokio::sync::Mutex;
use crate::config::config::AppConfig;
use crate::services::udp_listener::UdpListener;

#[derive(Clone)]
struct AppState {
    bot: Bot,
    chat_ids: Arc<Mutex<Vec<i64>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let bot_token = env::var("TELOXIDE_TOKEN")
        .with_context(|| "Переменная окружения TELOXIDE_TOKEN не задана")?;


    let cfg = AppConfig::load()?;

    let udp_port = cfg.server.port;

    let bot = Bot::new(bot_token);

    let chat_ids = Arc::new(Mutex::new(Vec::new()));
    let state = AppState {
        bot: bot.clone(),
        chat_ids: chat_ids.clone(),
    };

    let udp_bot = bot.clone();
    let udp_chat_ids = chat_ids.clone();
    tokio::spawn(async move {
        match UdpListener::new(cfg.addr(), udp_bot, udp_chat_ids).await {
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

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handle_message));

    println!("Telegram бот запущен! Ожидаем сообщения...");
    println!("UDP сервер слушает на порту: {}", udp_port);
    println!("Отправьте любое сообщение боту, чтобы активировать уведомления");

    Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![state.clone()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn handle_message(bot: Bot, msg: Message, state: AppState) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    {
        let mut ids = state.chat_ids.lock().await;
        if !ids.contains(&chat_id) {
            ids.push(chat_id);
            println!("📱 Новый чат добавлен: {} (ID: {})",
                     msg.chat.username().unwrap_or("без username"),
                     chat_id
            );
        }
    }

    bot.send_message(
        msg.chat.id,
        "Бот активирован! Вы будете получать уведомления с устройства.\n\
        📡 Данные принимаются на UDP порту 8080",
    )
        .await?;

    Ok(())
}
