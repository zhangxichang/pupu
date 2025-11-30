pub mod types;

use parking_lot::Mutex;
use rusqlite::{hooks::Action, params_from_iter};
use tauri::ipc::Channel;

use crate::{
    error::{Error, OptionGet},
    sqlite::types::{SQLiteType, SQLiteUpdateEvent},
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
pub async fn sqlite_on_update(
    state: tauri::State<'_, State>,
    channel: Channel<SQLiteUpdateEvent>,
) -> Result<(), Error> {
    state.db.connection.lock().get()?.update_hook(Some(
        move |update_type: Action, db_name: &str, table_name: &str, row_id| {
            if let Err(err) = channel.send(SQLiteUpdateEvent {
                update_type: match update_type {
                    Action::SQLITE_DELETE => 9,
                    Action::SQLITE_INSERT => 18,
                    Action::SQLITE_UPDATE => 23,
                    _ => -1,
                },
                db_name: db_name.to_string(),
                table_name: table_name.to_string(),
                row_id,
            }) {
                log::error!("{}", err);
            }
        },
    ));
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
    state.db.connection.lock().get()?.execute(
        &sql,
        params_from_iter(
            params
                .into_iter()
                .map(|value| value.try_into())
                .collect::<Result<Vec<SQLiteType>, Error>>()?,
        ),
    )?;
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
        .query_map(
            params_from_iter(
                params
                    .into_iter()
                    .map(|value| value.try_into())
                    .collect::<Result<Vec<SQLiteType>, Error>>()?,
            ),
            |row| {
                let mut object = serde_json::Map::new();
                for key in row.as_ref().column_names() {
                    let index = row.as_ref().column_index(key)?;
                    let data_type = row.get_ref(index)?.data_type();
                    object.insert(
                        key.to_string(),
                        row.get::<_, SQLiteType>(index)?
                            .try_into()
                            .map_err(|err: Error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    index,
                                    data_type,
                                    err.into(),
                                )
                            })?,
                    );
                }
                Ok(serde_json::Value::Object(object))
            },
        )?
        .collect::<Result<_, _>>()?)
}
