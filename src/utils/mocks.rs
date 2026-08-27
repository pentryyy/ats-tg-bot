use crate::traits::user_collector::UserCollectorTrait;
use anyhow::Result;
use async_trait::async_trait;
use mockall::mock;
use std::sync::Arc;

mock! {
    pub UserCollector {}

    #[async_trait]
    impl UserCollectorTrait for UserCollector {
        async fn start_collecting(&self) -> Result<()>;
        async fn get_active_ids(&self) -> Arc<Vec<String>>;
        async fn get_stats(&self) -> Result<(i64, i64)>;
        async fn add_user_from_telegram(&self, chat_id: &str);
        async fn deactivate_user(&self, chat_id: &str);
        async fn is_user_active(&self, chat_id: &str) -> bool;
    }
}
