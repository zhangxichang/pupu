use eyre::Result;

use crate::router::sqlite_api::traits::IntoSQLiteValue;

pub fn json_sql_params(params: Vec<serde_json::Value>) -> Result<Vec<rusqlite::types::Value>> {
    params
        .into_iter()
        .map(IntoSQLiteValue::into_sqlite_value)
        .collect()
}
