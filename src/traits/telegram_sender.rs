use async_trait::async_trait;
use teloxide::RequestError;
use teloxide::types::{ChatId, InputFile, Message};

#[async_trait]
pub trait TelegramSenderTrait: Send + Sync {
    async fn send_picture(&self, chat_id: ChatId, file: InputFile)
    -> Result<Message, RequestError>;
}
