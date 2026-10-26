pub mod client;
pub mod worker;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageRequest {
    Execute {
        sql: String,
        params: Vec<serde_json::Value>,
    },
    Query {
        sql: String,
        params: Vec<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoragePayload {
    Ok,
    Rows(QueryResult),
    Error(String),
}
