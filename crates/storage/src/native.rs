#![cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]

use std::path::PathBuf;

use eyre::{Result, WrapErr};
use rusqlite::{Connection, params_from_iter, types::ValueRef};
use sea_query::OptionEnum;
use storage_migrations::MIGRATIONS;

use crate::{QueryResult, SqlValue, Statement, StorageBackend};

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
        tokio::task::block_in_place(|| {
            let mut connection = rusqlite::Connection::open(data_dir.join("data.db"))?;
            connection.pragma_update(None, "foreign_keys", "OFF")?;
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "NORMAL")?;
            connection.pragma_update(None, "temp_store", "MEMORY")?;
            connection.pragma_update(None, "busy_timeout", 5000)?;
            connection.pragma_update(None, "cache_size", -64000)?;
            MIGRATIONS.to_latest(&mut connection)?;
            connection.pragma_update(None, "foreign_keys", "ON")?;
            Ok(Self { connection })
        })
    }
}

fn to_sql_value(value: &sea_query::Value) -> Result<rusqlite::types::Value> {
    Ok(match value {
        sea_query::Value::Bool(Some(value)) => {
            rusqlite::types::Value::Integer(if *value { 1 } else { 0 })
        }
        sea_query::Value::TinyInt(Some(value)) => {
            rusqlite::types::Value::Integer(i64::from(*value))
        }
        sea_query::Value::SmallInt(Some(value)) => {
            rusqlite::types::Value::Integer(i64::from(*value))
        }
        sea_query::Value::Int(Some(value)) => rusqlite::types::Value::Integer(i64::from(*value)),
        sea_query::Value::BigInt(Some(value)) => rusqlite::types::Value::Integer(*value),
        sea_query::Value::TinyUnsigned(Some(value)) => {
            rusqlite::types::Value::Integer(i64::from(*value))
        }
        sea_query::Value::SmallUnsigned(Some(value)) => {
            rusqlite::types::Value::Integer(i64::from(*value))
        }
        sea_query::Value::Unsigned(Some(value)) => {
            rusqlite::types::Value::Integer(i64::from(*value))
        }
        sea_query::Value::BigUnsigned(Some(value)) => {
            rusqlite::types::Value::Integer(i64::try_from(*value).wrap_err("整数超出 SQLite 范围")?)
        }
        sea_query::Value::Float(Some(value)) => rusqlite::types::Value::Real(f64::from(*value)),
        sea_query::Value::Double(Some(value)) => rusqlite::types::Value::Real(*value),
        sea_query::Value::String(Some(value)) => rusqlite::types::Value::Text(value.clone()),
        sea_query::Value::Enum(OptionEnum::Some(value)) => {
            rusqlite::types::Value::Text(value.value.to_string())
        }
        sea_query::Value::Char(Some(value)) => rusqlite::types::Value::Text(value.to_string()),
        sea_query::Value::Bytes(Some(value)) => rusqlite::types::Value::Blob(value.clone()),
        sea_query::Value::Enum(OptionEnum::None(_))
        | sea_query::Value::Bool(None)
        | sea_query::Value::TinyInt(None)
        | sea_query::Value::SmallInt(None)
        | sea_query::Value::Int(None)
        | sea_query::Value::BigInt(None)
        | sea_query::Value::TinyUnsigned(None)
        | sea_query::Value::SmallUnsigned(None)
        | sea_query::Value::Unsigned(None)
        | sea_query::Value::BigUnsigned(None)
        | sea_query::Value::Float(None)
        | sea_query::Value::Double(None)
        | sea_query::Value::String(None)
        | sea_query::Value::Char(None)
        | sea_query::Value::Bytes(None) => rusqlite::types::Value::Null,
    })
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

fn execute_statement(connection: &Connection, statement: &Statement) -> Result<u64> {
    let params = statement
        .values
        .0
        .iter()
        .map(to_sql_value)
        .collect::<Result<Vec<_>>>()?;
    let rows = connection
        .prepare(&statement.sql)?
        .execute(params_from_iter(params))?;
    Ok(rows as u64)
}

fn query_statement(connection: &Connection, statement: &Statement) -> Result<QueryResult> {
    let mut prepared = connection.prepare(&statement.sql)?;
    let columns = prepared
        .column_names()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();
    let params = statement
        .values
        .0
        .iter()
        .map(to_sql_value)
        .collect::<Result<Vec<_>>>()?;
    let mut rows = prepared.query(params_from_iter(params))?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let mut values = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            values.push(from_value_ref(row.get_ref(index)?));
        }
        result.push(values);
    }
    Ok(QueryResult {
        columns,
        rows: result,
    })
}

fn execute_batch_statement(connection: &mut Connection, statements: &[Statement]) -> Result<()> {
    let transaction = connection.transaction()?;
    for statement in statements {
        execute_statement(&transaction, statement)?;
    }
    transaction.commit()?;
    Ok(())
}

impl StorageBackend for Storage {
    async fn execute(&mut self, statement: Statement) -> Result<u64> {
        tokio::task::block_in_place(|| execute_statement(&self.connection, &statement))
    }
    async fn query(&mut self, statement: Statement) -> Result<QueryResult> {
        tokio::task::block_in_place(|| query_statement(&self.connection, &statement))
    }
    async fn execute_batch(&mut self, statements: Vec<Statement>) -> Result<()> {
        tokio::task::block_in_place(|| execute_batch_statement(&mut self.connection, &statements))
    }
    async fn execute_returning(&mut self, statement: Statement) -> Result<QueryResult> {
        tokio::task::block_in_place(|| query_statement(&self.connection, &statement))
    }
}
