//! Proxy request logs DAO
//!
//! 为测速/诊断提供“近期成功请求”的统计（不触发真实请求，避免浪费 token）。

use crate::error::AppError;
use rusqlite::params_from_iter;

use super::super::{lock_conn, Database};

/// 近期成功请求统计（跨多个 provider_id 聚合）
#[derive(Debug, Clone)]
pub struct RecentSuccessStats {
    pub sample_count: usize,
    pub median_latency_ms: u64,
    pub last_success_at: i64,
    pub last_model: Option<String>,
}

impl Database {
    fn sanitize_gpt_model_name_for_display(id: &str) -> String {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return trimmed.to_string();
        }
        let lower = trimmed.to_lowercase();
        if !lower.starts_with("gpt-") {
            return trimmed.to_string();
        }
        fn is_all_digits(s: &str) -> bool {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
        }
        fn is_year(s: &str) -> bool {
            if s.len() != 4 || !is_all_digits(s) {
                return false;
            }
            matches!(s.parse::<u32>(), Ok(y) if (2000..=2099).contains(&y))
        }
        fn is_mm_or_dd(s: &str) -> bool {
            s.len() == 2 && is_all_digits(s)
        }
        fn is_yyyymmdd(s: &str) -> bool {
            s.len() == 8 && is_all_digits(s) && s.starts_with("20")
        }
        fn is_mmdd_code(s: &str) -> bool {
            if s.len() != 4 || !is_all_digits(s) {
                return false;
            }
            let mm = s[0..2].parse::<u32>().ok();
            let dd = s[2..4].parse::<u32>().ok();
            matches!((mm, dd), (Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d))
        }
        let parts: Vec<&str> = trimmed.split('-').collect();
        for i in 0..parts.len() {
            if is_yyyymmdd(parts[i]) {
                return parts[..i].join("-");
            }
            if is_year(parts[i]) {
                if i + 2 < parts.len() && is_mm_or_dd(parts[i + 1]) && is_mm_or_dd(parts[i + 2])
                {
                    return parts[..i].join("-");
                }
                return parts[..i].join("-");
            }
            if is_mmdd_code(parts[i]) {
                return parts[..i].join("-");
            }
        }
        if let Some(idx) = lower.find("-202") {
            return trimmed[..idx].to_string();
        }
        trimmed.to_string()
    }

    /// 获取近期成功请求统计（status_code 2xx）
    ///
    /// - `provider_ids` 允许传多个 provider_id（同一 supplier 的不同 provider）
    /// - `max_rows` 越小越“近期”，默认建议 10~30
    /// - `max_age_secs` 为 Some 时，过滤 created_at >= now - max_age_secs
    pub fn get_recent_success_stats(
        &self,
        provider_ids: &[String],
        app_type: &str,
        max_rows: usize,
        max_age_secs: Option<i64>,
    ) -> Result<Option<RecentSuccessStats>, AppError> {
        if provider_ids.is_empty() || max_rows == 0 {
            return Ok(None);
        }

        let max_rows = max_rows.min(200);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|e| AppError::Database(format!("读取系统时间失败: {e}")))?
            .as_secs() as i64;
        let min_created_at = max_age_secs
            .map(|s| now.saturating_sub(s.max(0)))
            .unwrap_or(0);

        let placeholders = std::iter::repeat("?")
            .take(provider_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            "SELECT latency_ms, model, created_at
             FROM proxy_request_logs
             WHERE app_type = ?
               AND provider_id IN ({placeholders})
               AND status_code >= 200 AND status_code < 300
               AND created_at >= ?
             ORDER BY created_at DESC
             LIMIT ?"
        );

        let conn = lock_conn!(self.conn);

        // params: app_type + provider_ids... + min_created_at + limit
        let mut all_params: Vec<rusqlite::types::Value> = Vec::with_capacity(provider_ids.len() + 3);
        all_params.push(rusqlite::types::Value::from(app_type.to_string()));
        for pid in provider_ids {
            all_params.push(rusqlite::types::Value::from(pid.to_string()));
        }
        all_params.push(rusqlite::types::Value::from(min_created_at));
        all_params.push(rusqlite::types::Value::from(max_rows as i64));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut rows = stmt
            .query(params_from_iter(all_params.iter()))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut latencies: Vec<u64> = Vec::new();
        let mut last_success_at: i64 = 0;
        let mut last_model: Option<String> = None;

        while let Some(r) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let latency_ms: i64 = r.get(0).map_err(|e| AppError::Database(e.to_string()))?;
            let model: String = r.get(1).map_err(|e| AppError::Database(e.to_string()))?;
            let created_at: i64 = r.get(2).map_err(|e| AppError::Database(e.to_string()))?;

            if latency_ms >= 0 {
                latencies.push(latency_ms as u64);
            }
            if last_success_at == 0 {
                last_success_at = created_at;
                last_model = Some(Self::sanitize_gpt_model_name_for_display(&model));
            }
        }

        if latencies.is_empty() {
            return Ok(None);
        }

        latencies.sort_unstable();
        let median = latencies[latencies.len() / 2];

        Ok(Some(RecentSuccessStats {
            sample_count: latencies.len(),
            median_latency_ms: median,
            last_success_at,
            last_model,
        }))
    }
}
