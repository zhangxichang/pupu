pub mod types;

use parking_lot::Mutex;
use rusqlite::{hooks::Action, params_from_iter};
use tauri::ipc::Channel;

use crate::{
    api::{
        Api,
        sqlite::types::{SQLiteType, SQLiteUpdateEvent},
    },
    error::Error,
    option_ext::OptionGet,
};

#[derive(Default)]
pub struct Sqlite {
    inner: Mutex<Option<rusqlite::Connection>>,
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_open(api: tauri::State<'_, Api>, path: String) -> Result<(), Error> {
    api.sqlite
        .inner
        .lock()
        .replace(rusqlite::Connection::open(path)?);
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_is_open(api: tauri::State<'_, Api>) -> Result<bool, Error> {
    Ok(api.sqlite.inner.lock().is_some())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_close(api: tauri::State<'_, Api>) -> Result<(), Error> {
    if let Some(db) = api.sqlite.inner.lock().take() {
        db.close().map_err(|v| v.1)?;
    }
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_on_update(
    api: tauri::State<'_, Api>,
    channel: Channel<SQLiteUpdateEvent>,
) -> Result<(), Error> {
    api.sqlite.inner.lock().get()?.update_hook(Some(
        move |action: Action, db_name: &str, table_name: &str, row_id| {
            if let Err(err) = channel.send(SQLiteUpdateEvent {
                update_type: match action {
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
pub async fn sqlite_execute_batch(api: tauri::State<'_, Api>, sql: String) -> Result<(), Error> {
    api.sqlite.inner.lock().get()?.execute_batch(&sql)?;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn sqlite_execute(
    api: tauri::State<'_, Api>,
    sql: String,
    params: Vec<serde_json::Value>,
) -> Result<(), Error> {
    api.sqlite.inner.lock().get()?.execute(
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
    api: tauri::State<'_, Api>,
    sql: String,
    params: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, Error> {
    let db = api.sqlite.inner.lock();
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
