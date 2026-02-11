mod endpoint_api;
mod log;
mod sqlite_api;

use specta_typescript::BigIntExportBehavior;
use taurpc::{Router, Typescript};

use crate::router::{
    endpoint_api::{EndpointApi, EndpointApiImpl},
    log::{LogApi, LogApiImpl},
    sqlite_api::{SQLiteApi, SQLiteApiImpl},
};

pub fn router<R: tauri::Runtime>() -> Router<R> {
    Router::<R>::new()
        .export_config(Typescript::new().bigint(BigIntExportBehavior::BigInt))
        .merge(LogApiImpl::default().into_handler())
        .merge(SQLiteApiImpl::default().into_handler())
        .merge(EndpointApiImpl::default().into_handler())
}
