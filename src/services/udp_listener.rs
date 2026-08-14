use std::net::SocketAddr;
use crate::services::socket::SocketService;
use anyhow::Result;
use std::sync::Arc;
use teloxide::Bot;
use teloxide::payloads::SendDocumentSetters;
use teloxide::prelude::Requester;
use teloxide::types::ChatId;
use tokio::sync::Mutex;
use crate::dto::request::frame::FrameData;

pub struct UdpListener {
    socket_service: SocketService,
    bot: Bot,
    chat_ids: Arc<Mutex<Vec<i64>>>,
}

impl UdpListener {
    pub async fn new(
        addr: impl std::net::ToSocketAddrs,
        bot: Bot,
        chat_ids: Arc<Mutex<Vec<i64>>>,
    ) -> Result<Self> {
        let socket_service = SocketService::bind(addr).await?;
        Ok(Self {
            socket_service,
            bot,
            chat_ids,
        })
    }

    pub async fn start_listening(self) -> Result<()> {
        let mut buf = vec![0u8; 65535];

        println!("UDP сервер запущен и слушает на порту...");

        loop {
            // Явно указываем тип FrameData
            let result: Result<(FrameData, SocketAddr)> = self.socket_service.recv_from(&mut buf).await;

            match result {
                Ok((frame_data, addr)) => {
                    println!("Получены данные от {}: размер {} байт", addr, frame_data.frame.len());

                    let chat_ids = self.chat_ids.lock().await;
                    for chat_id in chat_ids.iter() {
                        let chat_id = ChatId(*chat_id);

                        let file = teloxide::types::InputFile::memory(frame_data.frame.clone());

                        if let Ok(text) = String::from_utf8(frame_data.frame.clone()) {
                            let _ = self.bot.send_message(chat_id, format!("Получены данные:\n{}", text)).await;
                        } else {
                            let _ = self.bot.send_document(chat_id, file)
                                .caption("Новые данные с устройства")
                                .await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Ошибка при получении данных: {}", e);
                }
            }
        }
    }
}