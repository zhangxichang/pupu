use std::cell::RefCell;

use gloo_worker::oneshot::oneshot;
use rusqlite::Connection;

use crate::storage::web::{StoragePayload, StorageRequest};

thread_local! {
    static CONN: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

#[oneshot]
pub async fn StorageWorker(_req: StorageRequest) -> StoragePayload {
    todo!()
}
