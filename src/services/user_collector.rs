use crate::config::config::AppConfig;
use crate::traits::database_repository::DatabaseRepositoryTrait;
use crate::traits::user_collector::UserCollectorTrait;
use anyhow::Result;
use async_trait::async_trait;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

pub struct UserCollector {
    cfg: AppConfig,
    repo: Arc<dyn DatabaseRepositoryTrait>,
    active_users: Arc<Mutex<Arc<Vec<String>>>>,
}

impl UserCollector {
    pub fn new(cfg: AppConfig, repo: Arc<dyn DatabaseRepositoryTrait>) -> Self {
        Self {
            cfg,
            repo,
            active_users: Arc::new(Mutex::new(Arc::new(Vec::new()))),
        }
    }

    pub async fn start_collecting(&self, cancel_token: CancellationToken) -> Result<()> {
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
                _ = cancel_token.cancelled() => {
                    info!("Коллектор пользователей завершает работу по сигналу");
                    return Ok(());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::config::test_config;
    use mockall::predicate::*;
    use mockall::*;
    use std::time::Duration;
    use tokio::time;

    mock! {
        pub DatabaseRepository {}

        #[async_trait]
        impl DatabaseRepositoryTrait for DatabaseRepository {
            async fn get_active_chat_ids(&self) -> Result<Vec<String>>;
            async fn deactivate_old_users(&self, timeout_minutes: i64) -> Result<u64>;
            async fn get_stats(&self) -> Result<(i64, i64)>;
            async fn cleanup_old_users(&self, days: i64) -> Result<u64>;
            async fn upsert_user_from_telegram(&self, chat_id: &str) -> Result<()>;
            async fn set_user_active(&self, chat_id: &str, active: bool) -> Result<()>;
            async fn is_user_active(&self, chat_id: &str) -> Result<bool>;
        }
    }

    #[tokio::test]
    async fn test_new() {
        let cfg = test_config();
        let repo = Arc::new(MockDatabaseRepository::new());
        let collector = UserCollector::new(cfg, repo);
        let active = collector.active_users.lock().await;
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn test_get_active_ids() {
        let cfg = test_config();
        let repo = Arc::new(MockDatabaseRepository::new());
        let collector = UserCollector::new(cfg, repo);
        {
            let mut guard = collector.active_users.lock().await;
            *guard = Arc::new(vec!["id1".to_string(), "id2".to_string()]);
        }
        let ids = collector.get_active_ids().await;
        assert_eq!(ids.as_ref(), &vec!["id1".to_string(), "id2".to_string()]);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let mut mock_repo = MockDatabaseRepository::new();
        mock_repo
            .expect_get_stats()
            .times(1)
            .returning(|| Ok((10, 5)));

        let cfg = test_config();
        let repo = Arc::new(mock_repo);
        let collector = UserCollector::new(cfg, repo);

        let stats = collector.get_stats().await.unwrap();
        assert_eq!(stats, (10, 5));
    }

    #[tokio::test]
    async fn test_add_user_from_telegram() {
        let mut mock_repo = MockDatabaseRepository::new();
        mock_repo
            .expect_upsert_user_from_telegram()
            .with(eq("test_chat"))
            .times(1)
            .returning(|_| Ok(()));

        let cfg = test_config();
        let repo = Arc::new(mock_repo);
        let collector = UserCollector::new(cfg, repo);

        collector.add_user_from_telegram("test_chat").await;
    }

    #[tokio::test]
    async fn test_deactivate_user() {
        let mut mock_repo = MockDatabaseRepository::new();
        mock_repo
            .expect_set_user_active()
            .with(eq("test_chat"), eq(false))
            .times(1)
            .returning(|_, _| Ok(()));

        let cfg = test_config();
        let repo = Arc::new(mock_repo);
        let collector = UserCollector::new(cfg, repo);

        collector.deactivate_user("test_chat").await;
    }

    #[tokio::test]
    async fn test_is_user_active() {
        let mut mock_repo = MockDatabaseRepository::new();
        mock_repo
            .expect_is_user_active()
            .with(eq("test_chat"))
            .times(1)
            .returning(|_| Ok(true));

        let cfg = test_config();
        let repo = Arc::new(mock_repo);
        let collector = UserCollector::new(cfg, repo);

        let active = collector.is_user_active("test_chat").await;
        assert!(active);
    }

    #[tokio::test]
    async fn test_start_collecting_initial_fetch() {
        time::pause();

        let mut mock_repo = MockDatabaseRepository::new();
        mock_repo
            .expect_get_active_chat_ids()
            .times(1)
            .returning(|| Ok(vec!["start1".to_string(), "start2".to_string()]));
        mock_repo.expect_deactivate_old_users().never();
        mock_repo.expect_cleanup_old_users().never();

        let cfg = test_config();
        let repo = Arc::new(mock_repo);
        let collector = Arc::new(UserCollector::new(cfg, repo));
        let cancel_token = CancellationToken::new();

        let collector_clone = collector.clone();
        let cancel_token_clone = cancel_token.clone();
        let handle = tokio::spawn(async move {
            collector_clone
                .start_collecting(cancel_token_clone)
                .await
                .unwrap();
        });

        time::advance(Duration::from_millis(10)).await;

        let active = collector.get_active_ids().await;
        assert_eq!(
            active.as_ref(),
            &vec!["start1".to_string(), "start2".to_string()]
        );

        cancel_token.cancel();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_start_collecting_periodic_tasks() {
        time::pause();

        let mut mock_repo = MockDatabaseRepository::new();

        let mut call_count = 0;
        mock_repo.expect_get_active_chat_ids().returning(move || {
            call_count += 1;
            if call_count == 1 {
                Ok(vec!["start".to_string()])
            } else {
                Ok(vec!["after_update".to_string()])
            }
        });

        mock_repo.expect_deactivate_old_users().returning(|_| Ok(5));

        mock_repo.expect_cleanup_old_users().returning(|_| Ok(0));

        let mut cfg = test_config();
        cfg.user_collector.update_interval = Duration::from_millis(100);
        cfg.user_collector.cleanup_interval = Duration::from_millis(200);

        let repo = Arc::new(mock_repo);
        let collector = Arc::new(UserCollector::new(cfg, repo));
        let cancel_token = CancellationToken::new();

        let collector_clone = collector.clone();
        let cancel_token_clone = cancel_token.clone();
        let handle = tokio::spawn(async move {
            collector_clone
                .start_collecting(cancel_token_clone)
                .await
                .unwrap();
        });

        tokio::task::yield_now().await;

        let active = collector.get_active_ids().await;
        assert_eq!(active.as_ref(), &vec!["start".to_string()]);

        time::advance(Duration::from_millis(150)).await;

        tokio::task::yield_now().await;

        let active = collector.get_active_ids().await;
        assert_eq!(active.as_ref(), &vec!["after_update".to_string()]);

        cancel_token.cancel();
        handle.await.unwrap();
    }
}
