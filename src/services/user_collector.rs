use crate::repositories::chat_users::DatabaseRepository;
use crate::traits::user_collector::UserCollectorTrait;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};

pub struct UserCollector {
    repo: Arc<DatabaseRepository>,
    active_users: Arc<Mutex<Vec<String>>>,
}

impl UserCollector {
    pub fn new(db: Arc<DatabaseRepository>) -> Self {
        Self {
            repo: db,
            active_users: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start_collecting(&self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            match self.repo.deactivate_old_users(5).await {
                Ok(affected) => {
                    if affected > 0 {
                        info!("Деактивировано {} пользователей", affected);
                    }
                }
                Err(e) => {
                    error!("Ошибка деактивации: {}", e);
                }
            }

            match self.repo.get_active_chat_ids().await {
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
                    if let Ok(deleted) = self.repo.cleanup_old_users(7).await {
                        if deleted > 0 {
                            info!("Удалено {} старых записей", deleted);
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl UserCollectorTrait for UserCollector {
    async fn get_active_ids(&self) -> Vec<String> {
        self.active_users.lock().await.clone()
    }

    async fn get_stats(&self) -> Result<(i64, i64)> {
        self.repo.get_stats().await.map_err(Into::into)
    }

    async fn add_user_from_telegram(&self, chat_id: &str) {
        if let Err(e) = self.repo.upsert_user_from_telegram(chat_id).await {
            error!("Ошибка регистрации пользователя {}: {}", chat_id, e);
        }
    }

    async fn deactivate_user(&self, chat_id: &str) {
        if let Err(e) = self.repo.set_user_active(chat_id, false).await {
            error!("Ошибка деактивации пользователя {}: {}", chat_id, e);
        }
    }

    async fn is_user_active(&self, chat_id: &str) -> bool {
        self.repo.is_user_active(chat_id).await.unwrap_or(false)
    }
}
