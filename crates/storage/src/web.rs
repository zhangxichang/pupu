#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use eyre::{Result, eyre};
use gloo_worker::oneshot::{Oneshot, OneshotBridge, OneshotSpawner};
use sea_query::OptionEnum;
use serde::{Deserialize, Serialize};

use crate::{QueryResult, SqlValue, Statement, StorageBackend};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatement {
    pub sql: String,
    pub params: Vec<SqlValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageRequest {
    Open { data_dir: String },
    Execute(StorageStatement),
    Query(StorageStatement),
    Batch { statements: Vec<StorageStatement> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoragePayload {
    Ok,
    Affected(u64),
    Rows(QueryResult),
    Error(String),
}

struct ClientStorageWorker;

impl Future for ClientStorageWorker {
    type Output = StoragePayload;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        unreachable!("ClientStorageWorker is only a bridge type parameter")
    }
}

impl Oneshot for ClientStorageWorker {
    type Input = StorageRequest;

    fn create(_input: Self::Input) -> Self {
        Self
    }
}

pub struct Storage {
    bridge: OneshotBridge<ClientStorageWorker>,
}
impl Storage {
    pub async fn open(worker_url: impl Into<String>, data_dir: PathBuf) -> Result<Self> {
        let worker_url = worker_url.into();
        let mut bridge = OneshotSpawner::<ClientStorageWorker>::new()
            .with_loader(true)
            .spawn(&worker_url);

        match bridge
            .run(StorageRequest::Open {
                data_dir: data_dir.to_string_lossy().into_owned(),
            })
            .await
        {
            StoragePayload::Ok => {}
            StoragePayload::Error(err) => return Err(eyre!(err)),
            _ => return Err(eyre!("unexpected response")),
        }

        Ok(Self { bridge })
    }
}

fn to_sql_value(value: &sea_query::Value) -> Result<SqlValue> {
    Ok(match value {
        sea_query::Value::Bool(Some(value)) => SqlValue::Integer(if *value { 1 } else { 0 }),
        sea_query::Value::TinyInt(Some(value)) => SqlValue::Integer(i64::from(*value)),
        sea_query::Value::SmallInt(Some(value)) => SqlValue::Integer(i64::from(*value)),
        sea_query::Value::Int(Some(value)) => SqlValue::Integer(i64::from(*value)),
        sea_query::Value::BigInt(Some(value)) => SqlValue::Integer(*value),
        sea_query::Value::TinyUnsigned(Some(value)) => SqlValue::Integer(i64::from(*value)),
        sea_query::Value::SmallUnsigned(Some(value)) => SqlValue::Integer(i64::from(*value)),
        sea_query::Value::Unsigned(Some(value)) => SqlValue::Integer(i64::from(*value)),
        sea_query::Value::BigUnsigned(Some(value)) => {
            SqlValue::Integer(i64::try_from(*value).map_err(|_| eyre!("整数超出 SQLite 范围"))?)
        }
        sea_query::Value::Float(Some(value)) => SqlValue::Real(f64::from(*value)),
        sea_query::Value::Double(Some(value)) => SqlValue::Real(*value),
        sea_query::Value::String(Some(value)) => SqlValue::Text(value.clone()),
        sea_query::Value::Enum(OptionEnum::Some(value)) => SqlValue::Text(value.value.to_string()),
        sea_query::Value::Char(Some(value)) => SqlValue::Text(value.to_string()),
        sea_query::Value::Bytes(Some(value)) => SqlValue::Blob(value.clone()),
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
        | sea_query::Value::Bytes(None) => SqlValue::Null,
    })
}

fn storage_statement(statement: Statement) -> Result<StorageStatement> {
    Ok(StorageStatement {
        sql: statement.sql,
        params: statement
            .values
            .0
            .iter()
            .map(to_sql_value)
            .collect::<Result<Vec<_>>>()?,
    })
}

impl Storage {
    async fn run_query(&mut self, statement: Statement) -> Result<QueryResult> {
        match self
            .bridge
            .run(StorageRequest::Query(storage_statement(statement)?))
            .await
        {
            StoragePayload::Rows(result) => Ok(result),
            StoragePayload::Error(err) => Err(eyre!(err)),
            _ => Err(eyre!("unexpected response")),
        }
    }
}

impl StorageBackend for Storage {
    async fn execute(&mut self, statement: Statement) -> Result<u64> {
        match self
            .bridge
            .run(StorageRequest::Execute(storage_statement(statement)?))
            .await
        {
            StoragePayload::Affected(rows) => Ok(rows),
            StoragePayload::Error(err) => Err(eyre!(err)),
            _ => Err(eyre!("unexpected response")),
        }
    }
    async fn query(&mut self, statement: Statement) -> Result<QueryResult> {
        self.run_query(statement).await
    }
    async fn execute_batch(&mut self, statements: Vec<Statement>) -> Result<()> {
        let statements = statements
            .into_iter()
            .map(storage_statement)
            .collect::<Result<Vec<_>>>()?;
        match self.bridge.run(StorageRequest::Batch { statements }).await {
            StoragePayload::Ok => Ok(()),
            StoragePayload::Error(err) => Err(eyre!(err)),
            _ => Err(eyre!("unexpected response")),
        }
    }
    async fn execute_returning(&mut self, statement: Statement) -> Result<QueryResult> {
        self.run_query(statement).await
    }
}
