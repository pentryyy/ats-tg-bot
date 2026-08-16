use crate::config::config::AppConfig;
use crate::repositories::chat_users::DatabaseRepository;
use crate::traits::user_collector::UserCollectorTrait;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::interval;

pub struct UserCollector {
    cfg: AppConfig,
    repo: Arc<DatabaseRepository>,
    active_users: Arc<Mutex<Arc<Vec<String>>>>,
}

impl UserCollector {
    pub fn new(cfg: AppConfig, repo: Arc<DatabaseRepository>) -> Self {
        Self {
            cfg,
            repo,
            active_users: Arc::new(Mutex::new(Arc::new(Vec::new()))),
        }
    }

    pub async fn start_collecting(&self) -> Result<()> {
        match self.repo.get_active_chat_ids().await {
            Ok(ids) => {
                let mut active = self.active_users.lock().await;
                *active = Arc::new(ids);
                debug!(
                    "Начальное количество активных пользователей: {}",
                    active.len()
                );
            }
            Err(e) => {
                error!(
                    "Ошибка получения начального списка активных пользователей: {}",
                    e
                );
            }
        }

        let update_interval = self.cfg.user_collector.update_interval;
        let mut interval_timer = interval(update_interval);

        let cleanup_interval = self.cfg.user_collector.cleanup_interval;
        let mut cleanup_timer = interval(cleanup_interval);

        loop {
            tokio::select! {
                _ = interval_timer.tick() => {
                    let deactivate_minutes = self.cfg.deactivate_after_minutes();
                    if let Ok(affected) = self.repo.deactivate_old_users(deactivate_minutes).await {
                        if affected > 0 {
                            info!("Деактивировано {} пользователей", affected);
                        }
                    }

                    if let Ok(ids) = self.repo.get_active_chat_ids().await {
                        let mut active = self.active_users.lock().await;
                        *active = Arc::new(ids);
                        debug!("Активных пользователей: {}", active.len());
                    } else {
                        error!("Ошибка получения пользователей");
                    }
                }
                _ = cleanup_timer.tick() => {
                    let cleanup_days = self.cfg.cleanup_after_days();
                    if let Ok(deleted) = self.repo.cleanup_old_users(cleanup_days).await {
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
    async fn get_active_ids(&self) -> Arc<Vec<String>> {
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
