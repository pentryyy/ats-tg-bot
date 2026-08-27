use crate::AppConfig;
use crate::traits::socket_service::SocketServiceTrait;
use crate::traits::telegram_sender::TelegramSenderTrait;
use crate::traits::user_collector::UserCollectorTrait;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile};
use tokio_util::sync::CancellationToken;

#[async_trait]
impl TelegramSenderTrait for Bot {
    /// Разное именование метода трейта отправки фото, чтобы не было конфликтов.
    async fn send_picture(
        &self,
        chat_id: ChatId,
        file: InputFile,
    ) -> Result<Message, teloxide::RequestError> {
        self.send_photo(chat_id, file).await
    }
}

fn is_image(data: &[u8]) -> bool {
    // JPEG: FF D8 FF
    if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
        return true;
    }
    // PNG: 89 50 4E 47
    if data.len() >= 4 && data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47 {
        return true;
    }
    // GIF: 47 49 46 38
    if data.len() >= 4 && data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46 && data[3] == 0x38 {
        return true;
    }
    false
}

pub struct UdpListener<S, T, C>
where
    S: SocketServiceTrait,
    T: TelegramSenderTrait,
    C: UserCollectorTrait,
{
    cfg: AppConfig,
    socket_service: S,
    bot: T,
    collector: Arc<C>,
}

impl<S, T, C> UdpListener<S, T, C>
where
    S: SocketServiceTrait,
    T: TelegramSenderTrait,
    C: UserCollectorTrait,
{
    pub async fn new(cfg: AppConfig, socket_service: S, bot: T, collector: Arc<C>) -> Self {
        Self {
            cfg,
            socket_service,
            bot,
            collector,
        }
    }

    pub async fn start_listening(self, cancel_token: CancellationToken) -> Result<()> {
        let mut recv_buf = self.cfg.recv_buf();

        loop {
            tokio::select! {
                result = self.socket_service.recv_frame(&mut recv_buf) => {
                    match result {
                        Ok((frame_data, addr)) => {
                            info!(
                                "Получены данные от {}: размер {} байт",
                                addr,
                                frame_data.frame.len()
                            );

                            let active_ids_arc = self.collector.get_active_ids().await;
                            if active_ids_arc.is_empty() {
                                warn!("Нет активных пользователей для отправки");
                                continue;
                            }
                            debug!("Отправка данных {} пользователям", active_ids_arc.len());

                            if !is_image(&frame_data.frame) {
                                info!("Данные не являются изображением, отправка пропущена");
                                continue;
                            }

                            let file_id = self
                                .upload_and_get_file_id(&active_ids_arc, &frame_data.frame)
                                .await;

                            if let Some(file_id) = file_id {
                                for chat_id in active_ids_arc
                                    .iter()
                                    .skip(1)
                                    .filter_map(|s| s.parse::<i64>().ok())
                                    .map(ChatId)
                                {
                                    if let Err(e) = self
                                        .bot
                                        .send_picture(chat_id, InputFile::file_id(file_id.clone()))
                                        .await
                                    {
                                        error!("Ошибка отправки: {}", e);
                                    }
                                }
                            } else {
                                warn!("Не удалось получить file_id, отправляем с копированием");
                                for chat_id in active_ids_arc
                                    .iter()
                                    .filter_map(|s| s.parse::<i64>().ok())
                                    .map(ChatId)
                                {
                                    let file = InputFile::memory(frame_data.frame.clone());
                                    if let Err(e) = self.bot.send_picture(chat_id, file).await {
                                        error!("Ошибка отправки: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => error!("Ошибка при получении данных: {}", e),
                    }
                }
                 _ = cancel_token.cancelled() => {
                    info!("Получен сигнал остановки, завершаем работу");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn upload_and_get_file_id(
        &self,
        active_ids: &Arc<Vec<String>>,
        data: &[u8],
    ) -> Option<String> {
        let first_chat_id = active_ids.first().and_then(|s| s.parse::<i64>().ok())?;
        let chat_id = ChatId(first_chat_id);
        let file = InputFile::memory(data.to_vec());

        let msg = self.bot.send_picture(chat_id, file).await.ok()?;
        let photo = msg.photo()?.first()?;
        Some(photo.file.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::config::test_config;
    use crate::dto::request::frame::FrameData;
    use crate::traits::socket_service::SocketServiceTrait;
    use crate::traits::user_collector::UserCollectorTrait;
    use anyhow::Result;
    use mockall::predicate::*;
    use mockall::*;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use teloxide::types::{
        Chat, ChatId, ChatKind, ChatPrivate, FileMeta, InputFile, MediaKind, MediaPhoto, Message,
        MessageCommon, MessageId, MessageKind, PhotoSize, User,
    };
    use tokio::time::Duration;
    use tokio_util::sync::CancellationToken;

    mock! {
        pub SocketService {}

        #[async_trait]
        impl SocketServiceTrait for SocketService {
            async fn recv_frame(&self, buf: &mut [u8]) -> Result<(FrameData, SocketAddr)>;
        }
    }

    mock! {
        pub TelegramSender {}

        #[async_trait]
        impl TelegramSenderTrait for TelegramSender {
            async fn send_picture(&self, chat_id: ChatId, file: InputFile) -> Result<Message, teloxide::RequestError>;
        }
    }

    mock! {
        pub UserCollector {}

        #[async_trait]
        impl UserCollectorTrait for UserCollector {
            async fn get_active_ids(&self) -> Arc<Vec<String>>;
            async fn get_stats(&self) -> Result<(i64, i64)>;
            async fn add_user_from_telegram(&self, chat_id: &str);
            async fn deactivate_user(&self, chat_id: &str);
            async fn is_user_active(&self, chat_id: &str) -> bool;
        }
    }

    fn make_photo_message(file_id: &str) -> Message {
        Message {
            id: MessageId(1),
            thread_id: None,
            date: chrono::Utc::now(),
            chat: Chat {
                id: ChatId(123),
                kind: ChatKind::Private(ChatPrivate {
                    username: None,
                    first_name: Some("Test".to_string()),
                    last_name: None,
                    bio: None,
                    has_private_forwards: None,
                    has_restricted_voice_and_video_messages: None,
                    emoji_status_custom_emoji_id: None,
                }),
                photo: None,
                pinned_message: None,
                message_auto_delete_time: None,
                has_hidden_members: false,
                has_aggressive_anti_spam_enabled: false,
            },
            via_bot: None,
            kind: MessageKind::Common(MessageCommon {
                from: Some(User {
                    id: UserId(123),
                    is_bot: false,
                    first_name: "Test".to_string(),
                    last_name: None,
                    username: None,
                    language_code: None,
                    is_premium: false,
                    added_to_attachment_menu: false,
                }),
                sender_chat: None,
                author_signature: None,
                forward: None,
                reply_to_message: None,
                edit_date: None,
                media_kind: MediaKind::Photo(MediaPhoto {
                    photo: vec![PhotoSize {
                        file: FileMeta {
                            id: file_id.to_string(),
                            unique_id: "".to_string(),
                            size: 100,
                        },
                        width: 100,
                        height: 100,
                    }],
                    caption: None,
                    caption_entities: vec![],
                    has_media_spoiler: false,
                    media_group_id: None,
                }),
                reply_markup: None,
                is_topic_message: false,
                is_automatic_forward: false,
                has_protected_content: false,
            }),
        }
    }

    fn mock_socket(data: Vec<u8>, addr: SocketAddr, times: usize) -> MockSocketService {
        let mut mock = MockSocketService::new();
        mock.expect_recv_frame()
            .times(times)
            .returning(move |_buf| {
                let frame = FrameData {
                    frame: data.clone(),
                };
                Ok((frame, addr))
            });
        mock
    }

    fn mock_socket_once(data: Vec<u8>, addr: SocketAddr) -> MockSocketService {
        let mut mock = MockSocketService::new();
        mock.expect_recv_frame().times(1).returning(move |_buf| {
            let frame = FrameData {
                frame: data.clone(),
            };
            Ok((frame, addr))
        });
        mock
    }

    #[tokio::test]
    async fn test_no_active_users() {
        tokio::time::pause();

        let non_image_data = b"hello world".to_vec();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mock_socket = mock_socket(non_image_data.clone(), addr, 1);

        let mut mock_collector = MockUserCollector::new();
        mock_collector
            .expect_get_active_ids()
            .times(1)
            .returning(|| Arc::new(vec![]));

        let mock_sender = MockTelegramSender::new();

        let cfg = test_config();
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        let listener =
            UdpListener::new(cfg, mock_socket, mock_sender, Arc::new(mock_collector)).await;

        let handle = tokio::spawn(async move {
            listener.start_listening(cancel_token_clone).await.unwrap();
        });

        tokio::time::advance(Duration::from_millis(10)).await;
        cancel_token.cancel();

        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok(), "Задача не завершилась вовремя");
    }
    #[tokio::test]
    async fn test_non_image_data_ignored() {
        tokio::time::pause();

        let non_image_data = b"hello world".to_vec();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let mock_socket = mock_socket_once(non_image_data.clone(), addr);
        let mut mock_collector = MockUserCollector::new();
        mock_collector
            .expect_get_active_ids()
            .times(1)
            .returning(|| Arc::new(vec!["123".to_string()]));

        let mut mock_sender = MockTelegramSender::new();
        mock_sender.expect_send_picture().times(0);

        let cfg = test_config();
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let listener =
            UdpListener::new(cfg, mock_socket, mock_sender, Arc::new(mock_collector)).await;

        let handle = tokio::spawn(async move {
            listener.start_listening(cancel_token_clone).await.unwrap();
        });

        tokio::time::advance(Duration::from_millis(10)).await;
        cancel_token.cancel();
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_successful_send_to_all() {
        tokio::time::pause();

        let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let mock_socket = mock_socket(image_data.clone(), addr, 1);
        let active_ids = vec!["111".to_string(), "222".to_string()];
        let mut mock_collector = MockUserCollector::new();
        mock_collector
            .expect_get_active_ids()
            .times(1)
            .returning(move || Arc::new(active_ids.clone()));

        let mut mock_sender = MockTelegramSender::new();
        mock_sender
            .expect_send_picture()
            .with(eq(ChatId(111)), always())
            .times(1)
            .returning(|_, _| Ok(make_photo_message("file_id_111")));
        mock_sender
            .expect_send_picture()
            .with(eq(ChatId(222)), always())
            .times(1)
            .returning(|_, _| Ok(make_photo_message("file_id_222")));

        mock_sender.expect_send_picture().times(0);

        let cfg = test_config();
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let listener =
            UdpListener::new(cfg, mock_socket, mock_sender, Arc::new(mock_collector)).await;

        let handle = tokio::spawn(async move {
            listener.start_listening(cancel_token_clone).await.unwrap();
        });

        tokio::time::advance(Duration::from_millis(10)).await;
        cancel_token.cancel();
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cancellation() {
        tokio::time::pause();

        let mut mock_socket = MockSocketService::new();
        mock_socket
            .expect_recv_frame()
            .times(1)
            .returning(move |_buf| Err(anyhow::anyhow!("test error")));

        let mock_collector = MockUserCollector::new();
        let mock_sender = MockTelegramSender::new();

        let cfg = test_config();
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let listener =
            UdpListener::new(cfg, mock_socket, mock_sender, Arc::new(mock_collector)).await;

        let handle = tokio::spawn(async move {
            listener.start_listening(cancel_token_clone).await.unwrap();
        });

        tokio::time::advance(Duration::from_millis(10)).await;
        cancel_token.cancel();

        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());
    }
}
