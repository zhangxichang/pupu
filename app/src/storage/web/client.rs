use std::path::PathBuf;

use gloo_worker::{Spawnable, oneshot::OneshotBridge};

use crate::storage::{
    Storage,
    web::{QueryResult, StorageCommand, StoragePayload, StorageRequest, worker::StorageWorker},
};

pub struct WebStorage {
    bridge: OneshotBridge<StorageWorker>,
    data_dir: String,
}

impl WebStorage {
    pub fn open(data_dir: PathBuf) -> Self {
        Self {
            bridge: StorageWorker::spawner().spawn("/storage.js"),
            data_dir: data_dir.to_string_lossy().into_owned(),
        }
    }

    pub async fn execute(
        &mut self,
        sql: impl Into<String>,
        params: Vec<serde_json::Value>,
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
        params: Vec<serde_json::Value>,
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

impl Storage for WebStorage {}
