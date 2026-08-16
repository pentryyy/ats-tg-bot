use anyhow::Result;
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Duration;

#[derive(Clone)]
pub struct DatabaseRepository {
    pool: PgPool,
}

impl DatabaseRepository {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn upsert_user(
        &self,
        chat_id: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO chat_users (chat_id, first_seen, last_seen, is_active, metadata)
            VALUES ($1, $2, $3, true, $4)
            ON CONFLICT (chat_id) DO UPDATE
            SET last_seen = $3,
                is_active = true,
                metadata = COALESCE($4, chat_users.metadata)
            "#,
        )
        .bind(chat_id)
        .bind(now)
        .bind(now)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_active_chat_ids(&self) -> Result<Vec<String>> {
        let users = sqlx::query(
            r#"
        SELECT chat_id
        FROM chat_users
        WHERE is_active = true
        AND last_seen > NOW() - INTERVAL '5 minutes'
        "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users
            .iter()
            .map(|row| row.try_get("chat_id"))
            .collect::<Result<Vec<String>, _>>()?)
    }

    pub async fn deactivate_old_users(&self, timeout_minutes: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE chat_users
            SET is_active = false
            WHERE is_active = true
            AND last_seen < NOW() - INTERVAL '1 minute' * $1
            "#,
        )
        .bind(timeout_minutes)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_stats(&self) -> Result<(i64, i64)> {
        let result = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE is_active = true) as active
            FROM chat_users
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = result.try_get("total")?;
        let active: i64 = result.try_get("active")?;

        Ok((total, active))
    }

    pub async fn cleanup_old_users(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM chat_users
            WHERE is_active = false
            AND last_seen < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
