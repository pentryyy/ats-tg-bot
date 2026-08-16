use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use anyhow::Result;
use log::{debug, error, info};
use crate::repositories::chat_users::DatabaseRepository;

pub struct UserCollector {
    db: Arc<DatabaseRepository>,
    active_users: Arc<Mutex<Vec<String>>>,
}

impl UserCollector {
    pub fn new(db: Arc<DatabaseRepository>) -> Self {
        Self {
            db,
            active_users: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start_collecting(&self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            match self.db.deactivate_old_users(5).await {
                Ok(affected) => {
                    if affected > 0 {
                        info!("Деактивировано {} пользователей", affected);
                    }
                }
                Err(e) => {
                    error!("Ошибка деактивации: {}", e);
                }
            }

            match self.db.get_active_chat_ids().await {
                Ok(ids) => {
                    let mut active = self.active_users.lock().await;
                    *active = ids;
                    debug!("Активных пользователей: {}", active.len());
                }
                Err(e) => {
                    error!("Ошибка получения пользователей: {}", e);
                }
            }

            static mut CLEANUP_COUNTER: u32 = 0;
            unsafe {
                CLEANUP_COUNTER += 1;
                if CLEANUP_COUNTER >= 120 {
                    CLEANUP_COUNTER = 0;
                    if let Ok(deleted) = self.db.cleanup_old_users(7).await {
                        if deleted > 0 {
                            info!("Удалено {} старых записей", deleted);
                        }
                    }
                }
            }
        }
    }

    pub async fn add_user_from_udp(&self, chat_id: &str, metadata: Option<serde_json::Value>) {
        if let Err(e) = self.db.upsert_user(chat_id, metadata).await {
            error!("Ошибка добавления пользователя {}: {}", chat_id, e);
        }
    }

    pub async fn get_active_ids(&self) -> Vec<String> {
        self.active_users.lock().await.clone()
    }

    pub async fn get_stats(&self) -> Result<(i64, i64)> {
        self.db.get_stats().await.map_err(Into::into)
    }
}