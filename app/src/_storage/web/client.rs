use gloo_worker::{Spawnable, oneshot::OneshotBridge};

use crate::storage::{Storage, web::worker::StorageWorker};

pub struct WebStorage {
    bridge: OneshotBridge<StorageWorker>,
}
impl WebStorage {
    pub fn new() -> Self {
        Self {
            bridge: StorageWorker::spawner().spawn("/storage.js"),
        }
    }
}
impl Storage for WebStorage {}
