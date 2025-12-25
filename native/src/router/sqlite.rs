mod traits;
mod types;
mod utils;

use std::sync::Arc;

use eyre::Result;
use parking_lot::Mutex;
use rusqlite::{hooks::Action, params_from_iter};
use tauri::ipc::Channel;

use crate::{
    option_ext::OptionGet,
    router::sqlite::{traits::IntoJSONValue, types::SQLiteUpdateEvent, utils::json_sql_params},
};

#[taurpc::procedures(path = "sqlite")]
pub trait SQLite {
    async fn open(path: String) -> Result<(), String>;
    async fn close() -> Result<(), String>;
    async fn execute_sql(sql: String) -> Result<(), String>;
    async fn execute(sql: String, params: Vec<serde_json::Value>) -> Result<(), String>;
    async fn query(
        sql: String,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String>;
    async fn on_update(channel: Channel<SQLiteUpdateEvent>) -> Result<(), String>;
}

#[derive(Clone, Default)]
pub struct SQLiteImpl {
    connection: Arc<Mutex<Option<rusqlite::Connection>>>,
}
#[taurpc::resolvers]
impl SQLite for SQLiteImpl {
    async fn open(self, path: String) -> Result<(), String> {
        async {
            self.connection
                .lock()
                .replace(rusqlite::Connection::open(path)?);
            eyre::Ok(())
        }
        .await
        .map_err(|err| err.to_string())
    }
    async fn close(self) -> Result<(), String> {
        async {
            if let Some(connection) = self.connection.lock().take() {
                connection.close().map_err(|(_, err)| err)?
            }
            eyre::Ok(())
        }
        .await
        .map_err(|err| err.to_string())
    }
    async fn execute_sql(self, sql: String) -> Result<(), String> {
        async {
            self.connection.lock().get()?.execute_batch(&sql)?;
            eyre::Ok(())
        }
        .await
        .map_err(|err| err.to_string())
    }
    async fn execute(self, sql: String, params: Vec<serde_json::Value>) -> Result<(), String> {
        async {
            self.connection
                .lock()
                .get()?
                .execute(&sql, params_from_iter(json_sql_params(params)?))?;
            eyre::Ok(())
        }
        .await
        .map_err(|err| err.to_string())
    }
    async fn query(
        self,
        sql: String,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        async {
            let connection = self.connection.lock();
            let connection = connection.get()?;
            let mut statement = connection.prepare(&sql)?;
            let column_names = statement
                .column_names()
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>();
            eyre::Ok(
                statement
                    .query_map(params_from_iter(json_sql_params(params)?), |row| {
                        let mut object = serde_json::Map::new();
                        for (i, key) in column_names.iter().enumerate() {
                            object.insert(
                                key.clone(),
                                row.get::<_, rusqlite::types::Value>(i)?.into_json_value(),
                            );
                        }
                        Ok(serde_json::Value::Object(object))
                    })?
                    .collect::<Result<_, _>>()?,
            )
        }
        .await
        .map_err(|err| err.to_string())
    }
    async fn on_update(self, channel: Channel<SQLiteUpdateEvent>) -> Result<(), String> {
        async {
            self.connection.lock().get()?.update_hook(Some(
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
            ))?;
            eyre::Ok(())
        }
        .await
        .map_err(|err| err.to_string())
    }
}
