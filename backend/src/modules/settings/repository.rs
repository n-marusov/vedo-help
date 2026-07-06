use std::collections::HashMap;

use sqlx::PgPool;

use crate::modules::settings::models::{SettingEntry, SettingRow};
use crate::shared::error::AppError;

/// Repository for the settings key-value table.
#[derive(Clone, Debug)]
pub struct SettingsRepository {
    db: PgPool,
}

impl SettingsRepository {
    /// Create a new SettingsRepository with the given database pool.
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Retrieve all settings as a key-value map.
    pub async fn get_all(&self) -> Result<HashMap<String, SettingEntry>, AppError> {
        tracing::debug!(
            component = "settings/repository",
            "settings.get_all.started"
        );

        let rows: Vec<SettingRow> = sqlx::query_as::<_, SettingRow>(
            "SELECT key, value, updated_at FROM settings ORDER BY key",
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to fetch settings: {e}")))?;

        let mut map = HashMap::new();
        for row in rows {
            let entry = row.into_entry();
            map.insert(entry.key.clone(), entry);
        }

        tracing::debug!(
            component = "settings/repository",
            count = map.len(),
            "settings.get_all.complete"
        );

        Ok(map)
    }

    /// Retrieve a single setting by key.
    pub async fn get(&self, key: &str) -> Result<Option<SettingEntry>, AppError> {
        tracing::debug!(component = "settings/repository", key = %key, "settings.get.started");

        let row: Option<SettingRow> = sqlx::query_as::<_, SettingRow>(
            "SELECT key, value, updated_at FROM settings WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get setting '{key}': {e}")))?;

        Ok(row.map(|r| r.into_entry()))
    }

    /// Upsert a single setting (insert or update).
    pub async fn upsert(&self, key: &str, value: &serde_json::Value) -> Result<(), AppError> {
        tracing::debug!(component = "settings/repository", key = %key, "settings.upsert.started");

        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, NOW()) \
             ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
        )
        .bind(key)
        .bind(value)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to upsert setting '{key}': {e}")))?;

        tracing::info!(component = "settings/repository", key = %key, "settings.upserted");
        Ok(())
    }

    /// Batch upsert multiple settings in a single transaction.
    pub async fn upsert_batch(
        &self,
        settings: &HashMap<String, serde_json::Value>,
    ) -> Result<(), AppError> {
        if settings.is_empty() {
            return Ok(());
        }

        tracing::debug!(
            component = "settings/repository",
            count = settings.len(),
            "settings.upsert_batch.started"
        );

        let mut tx =
            self.db.begin().await.map_err(|e| {
                AppError::InternalError(format!("Failed to begin transaction: {e}"))
            })?;

        for (key, value) in settings {
            sqlx::query(
                "INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, NOW()) \
                 ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()",
            )
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to upsert setting '{key}': {e}"))
            })?;
        }

        tx.commit().await.map_err(|e| {
            AppError::InternalError(format!("Failed to commit settings batch: {e}"))
        })?;

        tracing::info!(
            component = "settings/repository",
            count = settings.len(),
            "settings.upsert_batch.complete"
        );

        Ok(())
    }

    /// Delete a setting by key.
    pub async fn delete(&self, key: &str) -> Result<(), AppError> {
        tracing::debug!(component = "settings/repository", key = %key, "settings.delete.started");

        sqlx::query("DELETE FROM settings WHERE key = $1")
            .bind(key)
            .execute(&self.db)
            .await
            .map_err(|e| {
                AppError::InternalError(format!("Failed to delete setting '{key}': {e}"))
            })?;

        tracing::info!(component = "settings/repository", key = %key, "settings.deleted");
        Ok(())
    }
}
