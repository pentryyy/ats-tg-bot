use anyhow::{Context, Result};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{debug, error, info, warn};
use teloxide::prelude::*;
use teloxide::types::ChatId;
use crate::AppConfig;
use crate::dto::request::frame::FrameData;
use crate::services::socket::SocketService;
use crate::services::user_collector::UserCollector;

pub struct UdpListener {
    cfg: AppConfig,
    socket_service: SocketService,
    bot: Bot,
    collector: Arc<UserCollector>,
}

impl UdpListener {
    pub async fn new(
        cfg: AppConfig,
        collector: Arc<UserCollector>
    ) -> Result<Self> {
        let socket_service = SocketService::bind(cfg.service_addr()).await?;

        let bot_token = env::var("TELOXIDE_TOKEN")
            .with_context(|| "Переменная окружения TELOXIDE_TOKEN не задана")?;

        let bot = Bot::new(bot_token);

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

                    if let Some(chat_id) = self.parse_chat_id(&frame_data.frame) {
                        let metadata = serde_json::json!({
                            "source": addr.to_string(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "frame_size": frame_data.frame.len(),
                        });

                        self.collector
                            .add_user_from_udp(&chat_id, Some(metadata))
                            .await;

                        debug!("Добавлен/обновлен пользователь: {}", chat_id);
                    }

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

                        let file = teloxide::types::InputFile::memory(frame_data.frame.clone());

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

    fn parse_chat_id(&self, data: &[u8]) -> Option<String> {
        if let Ok(text) = String::from_utf8(data.to_vec()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() && trimmed.len() < 50 {
                return Some(trimmed.to_string());
            }
        }

        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
            if let Some(chat_id) = json
                .get("chat_id")
                .or_else(|| json.get("chat_id"))
                .and_then(|v| v.as_str())
            {
                return Some(chat_id.to_string());
            }
        }

        if let Ok(text) = String::from_utf8(data.to_vec()) {
            if let Some(captured) = text.split(':').nth(1) {
                let trimmed = captured.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }

        None
    }
}
