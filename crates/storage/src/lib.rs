pub mod accounts;
pub mod attachments;
pub mod contacts;
pub mod conversations;
pub mod messages;
pub mod native;
pub mod outbox;
pub mod sync;
pub mod web;

use std::future::Future;

use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<SqlValue>>,
}

#[derive(Debug, Clone)]
pub struct Statement {
    sql: String,
    values: sea_query::Values,
}
impl<SQL: Into<String>> From<(SQL, sea_query::Values)> for Statement {
    fn from(value: (SQL, sea_query::Values)) -> Self {
        Self {
            sql: value.0.into(),
            values: value.1,
        }
    }
}

pub trait StorageBackend {
    fn execute(&mut self, statement: Statement) -> impl Future<Output = Result<u64>>;
    fn query(&mut self, statement: Statement) -> impl Future<Output = Result<QueryResult>>;
    fn execute_batch(&mut self, statements: Vec<Statement>) -> impl Future<Output = Result<()>>;
    fn execute_returning(
        &mut self,
        statement: Statement,
    ) -> impl Future<Output = Result<QueryResult>>;
}
