use crate::services::user_collector::UserCollector;
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
    pub async fn command_handler(
        &self,
        bot: Bot,
        msg: Message,
        collector: Arc<UserCollector>,
    ) -> ResponseResult<()> {
        let chat_id_str = msg.chat.id.0.to_string();

        match self {
            AtsBotCommand::Start => {
                collector.add_user_from_telegram(&chat_id_str).await;
                bot.send_message(
                    msg.chat.id,
                    "✅ Вы успешно зарегистрированы для получения данных!",
                )
                .await?;
            }
            AtsBotCommand::Stop => {
                collector.deactivate_user(&chat_id_str).await;
                bot.send_message(msg.chat.id, "❌ Вы отписаны от получения данных.")
                    .await?;
            }
            AtsBotCommand::Status => {
                let is_active = collector.is_user_active(&chat_id_str).await;
                let status = if is_active {
                    "активен"
                } else {
                    "не активен"
                };
                bot.send_message(msg.chat.id, format!("📊 Ваш статус: {}", status))
                    .await?;
            }
        }
        Ok(())
    }
}
