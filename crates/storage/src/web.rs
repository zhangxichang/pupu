#![cfg(all(target_family = "wasm", target_os = "unknown"))]
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use eyre::{Result, eyre};
use gloo_worker::oneshot::{Oneshot, OneshotBridge, OneshotSpawner};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::StorageBackend;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCommand {
    pub data_dir: String,
    pub sql: String,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageRequest {
    Ping,
    Execute(StorageCommand),
    Query(StorageCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoragePayload {
    Ok,
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
    data_dir: String,
}
impl Storage {
    pub async fn open(worker_url: impl Into<String>, data_dir: PathBuf) -> Result<Self> {
        let worker_url = worker_url.into();
        let mut bridge = OneshotSpawner::<ClientStorageWorker>::new()
            .with_loader(true)
            .spawn(&worker_url);

        match bridge.run(StorageRequest::Ping).await {
            StoragePayload::Ok => {}
            StoragePayload::Error(err) => return Err(eyre!(err)),
            StoragePayload::Rows(_) => return Err(eyre!("unexpected Rows response")),
        }

        Ok(Self {
            bridge,
            data_dir: data_dir.to_string_lossy().into_owned(),
        })
    }
}

impl StorageBackend for Storage {}

impl Storage {
    pub async fn execute(
        &mut self,
        sql: impl Into<String>,
        params: Vec<Value>,
    ) -> eyre::Result<()> {
        match self
            .bridge
            .run(StorageRequest::Execute(StorageCommand {
                data_dir: self.data_dir.clone(),
                sql: sql.into(),
                params,
            }))
            .await
        {
            StoragePayload::Ok => Ok(()),
            StoragePayload::Error(err) => Err(eyre::eyre!(err)),
            StoragePayload::Rows(_) => Err(eyre::eyre!("unexpected Rows response")),
        }
    }
    pub async fn query(
        &mut self,
        sql: impl Into<String>,
        params: Vec<Value>,
    ) -> eyre::Result<QueryResult> {
        match self
            .bridge
            .run(StorageRequest::Query(StorageCommand {
                data_dir: self.data_dir.clone(),
                sql: sql.into(),
                params,
            }))
            .await
        {
            StoragePayload::Rows(result) => Ok(result),
            StoragePayload::Error(err) => Err(eyre::eyre!(err)),
            StoragePayload::Ok => Err(eyre::eyre!("unexpected Ok response")),
        }
    }
}
