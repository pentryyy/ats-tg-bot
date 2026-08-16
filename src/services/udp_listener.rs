use crate::AppConfig;
use crate::dto::request::frame::FrameData;
use crate::services::socket::SocketService;
use crate::services::user_collector::UserCollector;
use anyhow::Result;
use log::{debug, error, info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile};

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

                        if let Ok(text) = String::from_utf8(frame_data.frame.clone()) {
                            if let Err(e) = self
                                .bot
                                .send_message(chat_id, format!("Получены данные:\n{}", text))
                                .await
                            {
                                error!("Ошибка отправки сообщения {}: {}", chat_id, e);
                            }
                        } else {
                            if let Err(e) = self
                                .bot
                                .send_document(chat_id, file)
                                .caption("Новые данные с устройства")
                                .await
                            {
                                error!("Ошибка отправки документа {}: {}", chat_id, e);
                            }
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
