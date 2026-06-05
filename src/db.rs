#![cfg(feature = "database")]

use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

pub struct Database {
    pool: SqlitePool,
}

use crate::app::MetricSnapshot;

impl Database {
    pub async fn new() -> crate::error::Result<Self> {
        let mut db_path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        db_path.push("tempest-monitor");
        std::fs::create_dir_all(&db_path)?;
        db_path.push("metrics.db");

        let database_url = format!("sqlite:{}", db_path.to_string_lossy());
        
        // Ensure the file exists for sqlx
        if !db_path.exists() {
            std::fs::File::create(&db_path)?;
        }

        let pool = SqlitePool::connect(&database_url).await?;

        // Initialize schema
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                cpu_usage REAL,
                mem_used_gb REAL,
                gpu_usage REAL,
                net_rx_kbps REAL,
                net_tx_kbps REAL
            )"
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn save_snapshot(
        &self,
        cpu: f64,
        mem_gb: f64,
        gpu: f64,
        rx_kbps: f64,
        tx_kbps: f64,
    ) -> crate::error::Result<()> {
        sqlx::query(
            "INSERT INTO metrics (cpu_usage, mem_used_gb, gpu_usage, net_rx_kbps, net_tx_kbps)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(cpu)
        .bind(mem_gb)
        .bind(gpu)
        .bind(rx_kbps)
        .bind(tx_kbps)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn prune_old_data(&self, days: u32) -> crate::error::Result<u64> {
        let result = sqlx::query(
            "DELETE FROM metrics WHERE timestamp < datetime('now', ?)"
        )
        .bind(format!("-{} days", days))
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_recent_snapshots(&self, limit: u32) -> crate::error::Result<Vec<MetricSnapshot>> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, strftime('%Y-%m-%d %H:%M:%S', timestamp) AS timestamp, cpu_usage, mem_used_gb, gpu_usage, net_rx_kbps, net_tx_kbps FROM metrics ORDER BY id DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let snapshots = rows.into_iter().map(|row| {
            MetricSnapshot {
                id: row.get::<i64, _>("id"),
                timestamp: row.get::<String, _>("timestamp"),
                cpu_usage: row.get::<f64, _>("cpu_usage"),
                mem_used_gb: row.get::<f64, _>("mem_used_gb"),
                gpu_usage: row.get::<f64, _>("gpu_usage"),
                net_rx_kbps: row.get::<f64, _>("net_rx_kbps"),
                net_tx_kbps: row.get::<f64, _>("net_tx_kbps"),
            }
        }).collect();

        Ok(snapshots)
    }
}
