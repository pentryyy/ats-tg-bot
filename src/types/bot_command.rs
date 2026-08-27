use crate::traits::user_collector::UserCollectorTrait;
use log::info;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды")]
pub enum AtsBotCommand {
    #[command(description = "Зарегистрироваться для получения данных")]
    Start,
    #[command(description = "Отписаться от получения данных")]
    Stop,
    #[command(description = "Показать статус подписки")]
    Status,
}

impl AtsBotCommand {
    pub fn from_str(s: &str) -> Option<Self> {
        let s_trim = s.trim_start_matches('/');

        match s_trim {
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "status" => Some(Self::Status),
            _ => None,
        }
    }

    pub async fn command_handler(
        &self,
        bot: Bot,
        msg: Message,
        collector: Arc<dyn UserCollectorTrait>,
    ) -> ResponseResult<()> {
        let chat_id = msg.chat.id;
        let chat_id_str = chat_id.0.to_string();

        match self {
            AtsBotCommand::Start => {
                collector.add_user_from_telegram(&chat_id_str).await;
                bot.send_message(
                    chat_id,
                    "✅ Вы успешно зарегистрированы для получения данных!",
                )
                .await?;
                info!(
                    "Успешно зарегистрировано получение данных для chat id '{}'",
                    chat_id
                );
            }
            AtsBotCommand::Stop => {
                collector.deactivate_user(&chat_id_str).await;
                bot.send_message(chat_id, "❌ Вы отписаны от получения данных.")
                    .await?;
                info!("Отписано от получения данных для chat id '{}'", chat_id);
            }
            AtsBotCommand::Status => {
                let is_active = collector.is_user_active(&chat_id_str).await;
                let status = if is_active {
                    "активен"
                } else {
                    "не активен"
                };
                bot.send_message(chat_id, format!("📊 Ваш статус: {}", status))
                    .await?;
                info!("Статус для chat id '{}': {}", chat_id, status);
            }
        }
        Ok(())
    }
}
