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

                    let active_ids = self.collector.get_active_ids().await;

                    if active_ids.is_empty() {
                        warn!("Нет активных пользователей для отправки");
                        continue;
                    }

                    debug!("Отправка данных {} пользователям", active_ids.len());

                    for chat_id in active_ids {
                        let chat_id = match chat_id.parse::<i64>() {
                            Ok(id) => ChatId(id),
                            Err(_) => {
                                warn!("Некорректный chat_id: {}", chat_id);
                                continue;
                            }
                        };

                        let file = InputFile::memory(frame_data.frame.clone());
                        if is_image(&frame_data.frame) {
                            if let Err(e) = self.bot.send_photo(chat_id, file).await {
                                error!("Ошибка отправки фото для chat id '{}': {}", chat_id, e);
                            }
                        } else {
                            info!(
                                "Данные не являются изображением, отправка пропущена для chat id '{}'",
                                chat_id
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Ошибка при получении данных: {}", e);
                }
            }
        }
    }
}
