pub mod endpoint;
pub mod file_system;
pub mod log;
pub mod sqlite;

use crate::api::{endpoint::Endpoint, sqlite::Sqlite};

#[derive(Default)]
pub struct Api {
    pub sqlite: Sqlite,
    pub endpoint: Endpoint,
}
