#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use std::cell::RefCell;

use eyre::{Result, eyre};
use gloo_worker::{Registrable, oneshot::oneshot};
use rusqlite::{Connection, params_from_iter, types::ValueRef};
use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfgBuilder, install};
use storage::{
    QueryResult, SqlValue,
    web::{StoragePayload, StorageRequest, StorageStatement},
};
use storage_migrations::MIGRATIONS;
use wasm_bindgen::prelude::*;

struct Db {
    connection: Connection,
    data_dir: String,
}

thread_local! {
    static DB: RefCell<Option<Db>> = const { RefCell::new(None) };
}

async fn ensure_db(data_dir: &str) -> Result<()> {
    if let Some(current) = DB.with(|db| db.borrow().as_ref().map(|db| db.data_dir.clone())) {
        if current != data_dir {
            return Err(eyre!("database already open at a different directory"));
        }
        return Ok(());
    }

    let cfg = OpfsSAHPoolCfgBuilder::new().directory(data_dir).build();
    install::<sqlite_wasm_rs::WasmOsCallback>(&cfg, true).await?;

    let mut connection = Connection::open("pupu.db")?;
    connection.pragma_update(None, "foreign_keys", "OFF")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "busy_timeout", 5000)?;
    connection.pragma_update(None, "cache_size", -64000)?;
    MIGRATIONS.to_latest(&mut connection)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;

    DB.with(|cell| {
        *cell.borrow_mut() = Some(Db {
            connection,
            data_dir: data_dir.to_string(),
        });
    });
    Ok(())
}

fn to_sql_value(value: &SqlValue) -> rusqlite::types::Value {
    match value {
        SqlValue::Null => rusqlite::types::Value::Null,
        SqlValue::Integer(value) => rusqlite::types::Value::Integer(*value),
        SqlValue::Real(value) => rusqlite::types::Value::Real(*value),
        SqlValue::Text(value) => rusqlite::types::Value::Text(value.clone()),
        SqlValue::Blob(value) => rusqlite::types::Value::Blob(value.clone()),
    }
}

fn from_value_ref(value: ValueRef<'_>) -> SqlValue {
    match value {
        ValueRef::Null => SqlValue::Null,
        ValueRef::Integer(value) => SqlValue::Integer(value),
        ValueRef::Real(value) => SqlValue::Real(value),
        ValueRef::Text(value) => SqlValue::Text(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => SqlValue::Blob(value.to_vec()),
    }
}

fn execute(connection: &Connection, statement: &StorageStatement) -> Result<u64> {
    let mut stmt = connection.prepare(&statement.sql)?;
    let params = statement.params.iter().map(to_sql_value);
    let rows = stmt.execute(params_from_iter(params))?;
    Ok(rows as u64)
}

fn query(connection: &Connection, statement: &StorageStatement) -> Result<QueryResult> {
    let mut stmt = connection.prepare(&statement.sql)?;
    let columns = stmt
        .column_names()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    let column_count = columns.len();
    let params = statement.params.iter().map(to_sql_value);
    let mut rows = stmt.query(params_from_iter(params))?;
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

fn execute_batch(connection: &mut Connection, statements: &[StorageStatement]) -> Result<()> {
    let transaction = connection.transaction()?;
    for statement in statements {
        execute(&transaction, statement)?;
    }
    transaction.commit()?;
    Ok(())
}

async fn handle(request: StorageRequest) -> Result<StoragePayload> {
    if let StorageRequest::Open { data_dir } = &request {
        ensure_db(data_dir).await?;
        return Ok(StoragePayload::Ok);
    }

    DB.with(|cell| {
        let mut db = cell.borrow_mut();
        let db = db
            .as_mut()
            .ok_or_else(|| eyre!("database not initialized"))?;

        match request {
            StorageRequest::Open { .. } => Err(eyre!("unexpected Open request")),
            StorageRequest::Execute(statement) => Ok(StoragePayload::Affected(execute(
                &db.connection,
                &statement,
            )?)),
            StorageRequest::Query(statement) => {
                Ok(StoragePayload::Rows(query(&db.connection, &statement)?))
            }
            StorageRequest::Batch { statements } => {
                execute_batch(&mut db.connection, &statements)?;
                Ok(StoragePayload::Ok)
            }
        }
    })
}

#[oneshot]
pub async fn StorageWorker(request: StorageRequest) -> StoragePayload {
    match handle(request).await {
        Ok(payload) => payload,
        Err(err) => StoragePayload::Error(err.to_string()),
    }
}

#[wasm_bindgen]
pub fn register() {
    StorageWorker::registrar().register();
}
