use parking_lot::Mutex;
use rusqlite::params_from_iter;

use crate::{
    error::{Error, OptionExt},
    state::State,
};

#[derive(Default)]
pub struct Sqlite {
    connection: Mutex<Option<rusqlite::Connection>>,
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_open(state: tauri::State<'_, State>, path: String) -> Result<(), Error> {
    state
        .db
        .connection
        .lock()
        .replace(rusqlite::Connection::open(path)?);
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_is_open(state: tauri::State<'_, State>) -> Result<bool, Error> {
    Ok(state.db.connection.lock().is_some())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_close(state: tauri::State<'_, State>) -> Result<(), Error> {
    if let Some(db) = state.db.connection.lock().take() {
        db.close().map_err(|v| v.1)?;
    }
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_execute_batch(
    state: tauri::State<'_, State>,
    sql: String,
) -> Result<(), Error> {
    state.db.connection.lock().get()?.execute_batch(&sql)?;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_execute(
    state: tauri::State<'_, State>,
    sql: String,
    params: Vec<serde_json::Value>,
) -> Result<(), Error> {
    state
        .db
        .connection
        .lock()
        .get()?
        .execute(&sql, params_from_iter(params))?;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_query(
    state: tauri::State<'_, State>,
    sql: String,
    params: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, Error> {
    let db = state.db.connection.lock();
    let db = db.get()?;
    let mut statement = db.prepare(&sql)?;
    Ok(statement
        .query_map(params_from_iter(params), |row| {
            let mut object = serde_json::Map::new();
            for (i, key) in row.as_ref().column_names().iter().enumerate() {
                object.insert(key.to_string(), row.get(i)?);
            }
            Ok(serde_json::Value::Object(object))
        })?
        .collect::<Result<_, _>>()?)
}
