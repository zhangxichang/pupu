use std::path::PathBuf;

use eyre::Result;

use crate::storage::{Storage, migration::MIGRATIONS};

pub struct NativeStorage {
    connection: rusqlite::Connection,
}
impl NativeStorage {
    pub fn open(data_dir: PathBuf) -> Result<Self> {
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
impl Drop for NativeStorage {
    fn drop(&mut self) {
        _ = self.connection.execute_batch("PRAGMA optimize;");
    }
}
impl Storage for NativeStorage {}
