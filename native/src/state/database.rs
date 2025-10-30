use parking_lot::Mutex;
use tokio::fs::remove_file;

use crate::{error::Error, state::State, tauri_state_builder::TauriState};

#[derive(Default)]
pub struct Database {
    inner: Mutex<Option<duckdb::Connection>>,
}
impl TauriState for Database {
    fn tauri_builder<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
        builder.invoke_handler(tauri::generate_handler![
            state_database_init,
            state_database_delete
        ])
    }
}
#[tauri::command(rename_all = "snake_case")]
fn state_database_init(state: tauri::State<State>) -> Result<(), Error> {
    if state.database.inner.lock().is_none() {
        *state.database.inner.lock() = Some(duckdb::Connection::open("./data.db")?);
    }
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
async fn state_database_delete(state: tauri::State<'_, State>) -> Result<(), Error> {
    if let Some(database) = state.database.inner.lock().take() {
        database.close().map_err(|err| err.1)?;
    }
    remove_file("./data.db").await?;
    Ok(())
}
