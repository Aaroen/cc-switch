//! 供应商数据访问对象
//!
//! 提供供应商（Provider）的 CRUD 操作。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::provider::{Provider, ProviderMeta};
use indexmap::IndexMap;
use rusqlite::params;
use std::collections::HashMap;

impl Database {
    /// 获取指定应用类型的所有供应商
    pub fn get_all_providers(
        &self,
        app_type: &str,
    ) -> Result<IndexMap<String, Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, name, settings_config, website_url, category, created_at, sort_index, notes, icon, icon_color, meta
             FROM providers WHERE app_type = ?1
             ORDER BY COALESCE(sort_index, 999999), created_at ASC, id ASC"
        ).map_err(|e| AppError::Database(e.to_string()))?;

        let provider_iter = stmt
            .query_map(params![app_type], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let settings_config_str: String = row.get(2)?;
                let website_url: Option<String> = row.get(3)?;
                let category: Option<String> = row.get(4)?;
                let created_at: Option<i64> = row.get(5)?;
                let sort_index: Option<usize> = row.get(6)?;
                let notes: Option<String> = row.get(7)?;
                let icon: Option<String> = row.get(8)?;
                let icon_color: Option<String> = row.get(9)?;
                let meta_str: String = row.get(10)?;

                let settings_config =
                    serde_json::from_str(&settings_config_str).unwrap_or(serde_json::Value::Null);
                let meta: ProviderMeta = serde_json::from_str(&meta_str).unwrap_or_default();

                Ok((
                    id,
                    Provider {
                        id: "".to_string(), // Placeholder, set below
                        name,
                        settings_config,
                        website_url,
                        category,
                        created_at,
                        sort_index,
                        notes,
                        meta: Some(meta),
                        icon,
                        icon_color,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut providers = IndexMap::new();
        for provider_res in provider_iter {
            let (id, mut provider) = provider_res.map_err(|e| AppError::Database(e.to_string()))?;
            provider.id = id.clone();

            // 加载 endpoints
            let mut stmt_endpoints = conn.prepare(
                "SELECT url, added_at FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2 ORDER BY added_at ASC, url ASC"
            ).map_err(|e| AppError::Database(e.to_string()))?;

            let endpoints_iter = stmt_endpoints
                .query_map(params![id, app_type], |row| {
                    let url: String = row.get(0)?;
                    let added_at: Option<i64> = row.get(1)?;
                    Ok((
                        url,
                        crate::settings::CustomEndpoint {
                            url: "".to_string(),
                            added_at: added_at.unwrap_or(0),
                            last_used: None,
                        },
                    ))
                })
                .map_err(|e| AppError::Database(e.to_string()))?;

            let mut custom_endpoints = HashMap::new();
            for ep_res in endpoints_iter {
                let (url, mut ep) = ep_res.map_err(|e| AppError::Database(e.to_string()))?;
                ep.url = url.clone();
                custom_endpoints.insert(url, ep);
            }

            if let Some(meta) = &mut provider.meta {
                meta.custom_endpoints = custom_endpoints;
            }

            providers.insert(id, provider);
        }

        Ok(providers)
    }

    /// 获取当前激活的供应商 ID
    pub fn get_current_provider(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id FROM providers WHERE app_type = ?1 AND is_current = 1 LIMIT 1")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut rows = stmt
            .query(params![app_type])
            .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            Ok(Some(
                row.get(0).map_err(|e| AppError::Database(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    /// 保存供应商（新增或更新）
    pub fn save_provider(&self, app_type: &str, provider: &Provider) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 处理 meta 和 endpoints
        let mut meta_clone = provider.meta.clone().unwrap_or_default();
        let endpoints = std::mem::take(&mut meta_clone.custom_endpoints);

        // 检查是否存在以保留 is_current
        let is_current: bool = tx
            .query_row(
                "SELECT is_current FROM providers WHERE id = ?1 AND app_type = ?2",
                params![provider.id, app_type],
                |row| row.get(0),
            )
            .unwrap_or(false);

        tx.execute(
            "INSERT OR REPLACE INTO providers (
                id, app_type, name, settings_config, website_url, category,
                created_at, sort_index, notes, icon, icon_color, meta, is_current
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                provider.id,
                app_type,
                provider.name,
                serde_json::to_string(&provider.settings_config).unwrap(),
                provider.website_url,
                provider.category,
                provider.created_at,
                provider.sort_index,
                provider.notes,
                provider.icon,
                provider.icon_color,
                serde_json::to_string(&meta_clone).unwrap(),
                is_current,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 同步 endpoints：删除全部后重新插入
        tx.execute(
            "DELETE FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2",
            params![provider.id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        for (url, endpoint) in endpoints {
            tx.execute(
                "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![provider.id, app_type, url, endpoint.added_at],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 删除供应商
    pub fn delete_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 设置当前供应商
    pub fn set_current_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 重置所有为 0
        tx.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?1",
            params![app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 设置新的当前供应商
        tx.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit().map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 添加自定义端点
    pub fn add_custom_endpoint(
        &self,
        app_type: &str,
        provider_id: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let added_at = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at) VALUES (?1, ?2, ?3, ?4)",
            params![provider_id, app_type, url, added_at],
        ).map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 移除自定义端点
    pub fn remove_custom_endpoint(
        &self,
        app_type: &str,
        provider_id: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
            params![provider_id, app_type, url],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
