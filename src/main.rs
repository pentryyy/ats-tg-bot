mod config;
mod dto;
mod server;
mod services;
mod utils;

use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;
use teloxide::{Bot, prelude::*};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    bot: Bot,
    chat_ids: Arc<Mutex<Vec<i64>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let bot_token = env::var("TELOXIDE_TOKEN")
        .with_context(|| "Переменная окружения TELOXIDE_TOKEN не задана")?;

    let bot = Bot::new(bot_token);

    let state = AppState {
        bot: bot.clone(),
        chat_ids: Arc::new(Mutex::new(Vec::new())),
    };

    let handler = dptree::entry().branch(Update::filter_message().endpoint(handle_message));

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
            println!("Новый чат добавлен: {}", chat_id);
        }
    }

    bot.send_message(
        msg.chat.id,
        "Бот активирован! Вы будете получать уведомления.",
    )
    .await?;

    Ok(())
}
