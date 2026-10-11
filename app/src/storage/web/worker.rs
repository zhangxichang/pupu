use std::cell::RefCell;

use eyre::{Result, eyre};
use gloo_worker::oneshot::oneshot;
use rusqlite::Connection;
use rusqlite::types::{Value as SqlValue, ValueRef};
use serde_json::Value;
use sqlite_wasm_rs as ffi;
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfg, install};

use crate::storage::{
    migration::MIGRATIONS,
    web::{QueryResult, StoragePayload, StorageRequest},
};

thread_local! {
    static DB: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

async fn ensure_db() -> Result<()> {
    if DB.with(|db| db.borrow().is_some()) {
        return Ok(());
    }

    install::<ffi::WasmOsCallback>(&OpfsSAHPoolCfg::default(), true).await?;

    let mut connection = Connection::open("pupu.db")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "busy_timeout", 5000)?;
    MIGRATIONS.to_latest(&mut connection)?;

    DB.with(|cell| *cell.borrow_mut() = Some(connection));
    Ok(())
}

fn to_sql_value(value: &Value) -> Result<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Bool(value) => SqlValue::Integer(if *value { 1 } else { 0 }),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                SqlValue::Integer(value)
            } else if let Some(value) = value.as_f64() {
                SqlValue::Real(value)
            } else {
                return Err(eyre!("unsupported number parameter"));
            }
        }
        Value::String(value) => SqlValue::Text(value.clone()),
        Value::Array(values) => SqlValue::Blob(
            values
                .iter()
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|value| u8::try_from(value).ok())
                        .ok_or_else(|| eyre!("blob array must contain bytes"))
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        Value::Object(_) => return Err(eyre!("object parameters are not supported")),
    })
}

fn from_value_ref(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::from(value),
        ValueRef::Real(value) => Value::from(value),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => {
            Value::Array(value.iter().map(|byte| Value::from(*byte)).collect())
        }
    }
}

fn execute(connection: &Connection, sql: &str, params: &[Value]) -> Result<()> {
    if params.is_empty() {
        connection.execute_batch(sql)?;
        return Ok(());
    }

    let mut stmt = connection.prepare(sql)?;
    let params = params
        .iter()
        .map(to_sql_value)
        .collect::<Result<Vec<_>>>()?;
    stmt.execute(rusqlite::params_from_iter(params))?;
    Ok(())
}

fn query(connection: &Connection, sql: &str, params: &[Value]) -> Result<QueryResult> {
    let mut stmt = connection.prepare(sql)?;
    let columns = stmt
        .column_names()
        .iter()
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    let column_count = columns.len();
    let params = params
        .iter()
        .map(to_sql_value)
        .collect::<Result<Vec<_>>>()?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
    let mut result = Vec::new();

    while let Some(row) = rows.next()? {
        let mut values = Vec::with_capacity(column_count);
        for index in 0..column_count {
            values.push(from_value_ref(row.get_ref(index)?));
        }
        result.push(values);
    }

    Ok(QueryResult {
        columns,
        rows: result,
    })
}

async fn handle(request: StorageRequest) -> Result<StoragePayload> {
    ensure_db().await?;

    match request {
        StorageRequest::Execute { sql, params } => {
            DB.with(|cell| {
                let db = cell.borrow();
                let db = db
                    .as_ref()
                    .ok_or_else(|| eyre!("database not initialized"))?;
                execute(db, &sql, &params)
            })?;
            Ok(StoragePayload::Ok)
        }
        StorageRequest::Query { sql, params } => {
            let result = DB.with(|cell| {
                let db = cell.borrow();
                let db = db
                    .as_ref()
                    .ok_or_else(|| eyre!("database not initialized"))?;
                query(db, &sql, &params)
            })?;
            Ok(StoragePayload::Rows(result))
        }
    }
}

#[oneshot]
pub async fn StorageWorker(request: StorageRequest) -> StoragePayload {
    match handle(request).await {
        Ok(payload) => payload,
        Err(err) => StoragePayload::Error(err.to_string()),
    }
}
