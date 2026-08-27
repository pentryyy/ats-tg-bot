use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait DatabaseRepositoryTrait: Send + Sync {
    async fn get_active_chat_ids(&self) -> Result<Vec<String>>;
    async fn deactivate_old_users(&self, timeout_minutes: i64) -> Result<u64>;
    async fn get_stats(&self) -> Result<(i64, i64)>;
    async fn cleanup_old_users(&self, days: i64) -> Result<u64>;
    async fn upsert_user_from_telegram(&self, chat_id: &str) -> Result<()>;
    async fn set_user_active(&self, chat_id: &str, active: bool) -> Result<()>;
    async fn is_user_active(&self, chat_id: &str) -> Result<bool>;
}
