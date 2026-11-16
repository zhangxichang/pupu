#![cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]

use std::path::PathBuf;

use eyre::Result;

use storage_migrations::MIGRATIONS;

use crate::StorageBackend;

pub struct Storage {
    connection: rusqlite::Connection,
}
impl Drop for Storage {
    fn drop(&mut self) {
        _ = self.connection.execute_batch("PRAGMA optimize;");
    }
}
impl Storage {
    pub async fn open(data_dir: PathBuf) -> Result<Self> {
        let mut connection = rusqlite::Connection::open(data_dir.join("data.db"))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.pragma_update(None, "temp_store", "MEMORY")?;
        connection.pragma_update(None, "busy_timeout", 5000)?;
        connection.pragma_update(None, "cache_size", -64000)?;
        MIGRATIONS.to_latest(&mut connection)?;
        Ok(Self { connection })
    }
}
impl StorageBackend for Storage {}
