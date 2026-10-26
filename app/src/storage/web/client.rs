use gloo_worker::{Spawnable, oneshot::OneshotBridge};

use crate::storage::{
    Storage,
    web::{QueryResult, StoragePayload, StorageRequest, worker::StorageWorker},
};

pub struct WebStorage {
    bridge: OneshotBridge<StorageWorker>,
}

impl WebStorage {
    pub fn new() -> Self {
        Self {
            bridge: StorageWorker::spawner().spawn("/storage.js"),
        }
    }

    pub async fn execute(
        &mut self,
        sql: impl Into<String>,
        params: Vec<serde_json::Value>,
    ) -> eyre::Result<()> {
        match self
            .bridge
            .run(StorageRequest::Execute {
                sql: sql.into(),
                params,
            })
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
            .run(StorageRequest::Query {
                sql: sql.into(),
                params,
            })
            .await
        {
            StoragePayload::Rows(result) => Ok(result),
            StoragePayload::Error(err) => Err(eyre::eyre!(err)),
            StoragePayload::Ok => Err(eyre::eyre!("unexpected Ok response")),
        }
    }
}

impl Storage for WebStorage {}
