use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct ChatUser {
    pub id: i64,
    pub chat_id: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub is_active: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ActiveUser {
    pub chat_id: String,
    pub last_seen: DateTime<Utc>,
}
