use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
    ) -> Result<(), sqlx::Error> {
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

    pub async fn prune_old_data(&self, days: u32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM metrics WHERE timestamp < datetime('now', ?)"
        )
        .bind(format!("-{} days", days))
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
