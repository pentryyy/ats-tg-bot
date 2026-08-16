use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait UserCollectorTrait: Send + Sync {
    async fn get_active_ids(&self) -> Arc<Vec<String>>;
    async fn get_stats(&self) -> Result<(i64, i64)>;
    async fn add_user_from_telegram(&self, chat_id: &str);
    async fn deactivate_user(&self, chat_id: &str);
    async fn is_user_active(&self, chat_id: &str) -> bool;
}
