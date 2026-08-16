use crate::AppConfig;
use crate::dto::request::frame::FrameData;
use crate::services::socket::SocketService;
use crate::services::user_collector::UserCollector;
use crate::traits::user_collector::UserCollectorTrait;
use anyhow::Result;
use log::{debug, error, info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile};

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

pub struct UdpListener {
    cfg: AppConfig,
    socket_service: SocketService,
    bot: Bot,
    collector: Arc<UserCollector>,
}

impl UdpListener {
    pub async fn new(cfg: AppConfig, bot: Bot, collector: Arc<UserCollector>) -> Result<Self> {
        let socket_service = SocketService::bind(cfg.service_addr()).await?;

        Ok(Self {
            cfg,
            socket_service,
            bot,
            collector,
        })
    }

    pub async fn start_listening(self) -> Result<()> {
        let mut recv_buf = self.cfg.recv_buf();

        loop {
            let result: Result<(FrameData, SocketAddr)> =
                self.socket_service.recv_from(&mut recv_buf).await;

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
                                .send_photo(chat_id, InputFile::file_id(file_id.clone()))
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
                            if let Err(e) = self.bot.send_photo(chat_id, file).await {
                                error!("Ошибка отправки: {}", e);
                            }
                        }
                    }
                }
                Err(e) => error!("Ошибка при получении данных: {}", e),
            }
        }
    }

    /// Отправляет изображение первому активному пользователю и возвращает его `file_id`.
    /// Используется для оптимизации массовой рассылки: полученный `file_id` позволяет
    /// отправлять фото остальным пользователям без повторной передачи данных.
    /// Возвращает `Some(file_id)` при успехе, иначе `None`.
    async fn upload_and_get_file_id(
        &self,
        active_ids: &Arc<Vec<String>>,
        data: &[u8],
    ) -> Option<String> {
        let first_chat_id = active_ids.first().and_then(|s| s.parse::<i64>().ok())?;

        let chat_id = ChatId(first_chat_id);
        let file = InputFile::memory(data.to_vec());

        let msg = self.bot.send_photo(chat_id, file).await.ok()?;
        let photo = msg.photo()?.first()?;
        Some(photo.file.id.clone())
    }
}
