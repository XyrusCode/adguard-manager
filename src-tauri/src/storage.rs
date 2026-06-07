use crate::types::InterfaceSample;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use tracing::info;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let path = db_path();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).ok();
        }
        let conn = Connection::open(&path)?;
        let s = Self { conn };
        s.migrate()?;
        info!("storage: {:?}", path);
        Ok(s)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS network_samples (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                interface_name TEXT    NOT NULL,
                timestamp      TEXT    NOT NULL,
                bytes_received INTEGER NOT NULL,
                bytes_sent     INTEGER NOT NULL,
                rx_rate        REAL    NOT NULL,
                tx_rate        REAL    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ts   ON network_samples(timestamp);
            CREATE INDEX IF NOT EXISTS idx_iface ON network_samples(interface_name);",
        )
    }

    pub fn insert_sample(&self, s: &InterfaceSample) -> Result<()> {
        self.conn.execute(
            "INSERT INTO network_samples
             (interface_name, timestamp, bytes_received, bytes_sent, rx_rate, tx_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                s.interface_name,
                s.timestamp.to_rfc3339(),
                s.bytes_received as i64,
                s.bytes_sent as i64,
                s.rx_rate,
                s.tx_rate,
            ],
        )?;
        Ok(())
    }

    pub fn query_samples(
        &self,
        interface: Option<&str>,
        since: DateTime<Utc>,
    ) -> Result<Vec<InterfaceSample>> {
        let since_s = since.to_rfc3339();
        match interface {
            Some(iface) => {
                let mut st = self.conn.prepare(
                    "SELECT interface_name, timestamp, bytes_received, bytes_sent, rx_rate, tx_rate
                     FROM network_samples WHERE interface_name=?1 AND timestamp>=?2
                     ORDER BY timestamp ASC",
                )?;
                st.query_map(params![iface, since_s], map_row)?
                    .filter_map(|r| r.ok())
                    .collect()
            }
            None => {
                let mut st = self.conn.prepare(
                    "SELECT interface_name, timestamp, bytes_received, bytes_sent, rx_rate, tx_rate
                     FROM network_samples WHERE timestamp>=?1
                     ORDER BY timestamp ASC",
                )?;
                st.query_map(params![since_s], map_row)?
                    .filter_map(|r| r.ok())
                    .collect()
            }
        }
    }

    pub fn prune(&self, keep_days: i64) -> Result<usize> {
        let cut = (Utc::now() - chrono::Duration::days(keep_days)).to_rfc3339();
        self.conn
            .execute("DELETE FROM network_samples WHERE timestamp<?1", params![cut])
    }
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<InterfaceSample> {
    let ts: String = row.get(1)?;
    let timestamp = DateTime::parse_from_rfc3339(&ts)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Ok(InterfaceSample {
        interface_name: row.get(0)?,
        timestamp,
        bytes_received: row.get::<_, i64>(2)? as u64,
        bytes_sent: row.get::<_, i64>(3)? as u64,
        rx_rate: row.get(4)?,
        tx_rate: row.get(5)?,
    })
}

fn db_path() -> PathBuf {
    ProjectDirs::from("com", "adguard", "manager")
        .map(|d| d.data_local_dir().join("data.db"))
        .unwrap_or_else(|| PathBuf::from("adguard_manager.db"))
}
