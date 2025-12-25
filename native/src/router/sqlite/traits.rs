use eyre::{Result, bail};

use crate::option_ext::OptionGet;

pub trait IntoSQLiteValue {
    fn into_sqlite_value(self) -> Result<rusqlite::types::Value>;
}
impl IntoSQLiteValue for serde_json::Value {
    fn into_sqlite_value(self) -> Result<rusqlite::types::Value> {
        match self {
            serde_json::Value::Null => Ok(rusqlite::types::Value::Null),
            serde_json::Value::Bool(value) => Ok(value.into()),
            serde_json::Value::Number(value) => Ok(value.as_i64().get_move()?.into()),
            serde_json::Value::String(value) => Ok(value.into()),
            serde_json::Value::Array(value) => Ok(value
                .into_iter()
                .map(|v| Ok(v.as_u64().get_move()? as _))
                .collect::<Result<Vec<u8>>>()?
                .into()),
            serde_json::Value::Object(_) => bail!("目标不能为对象"),
        }
    }
}

pub trait IntoJSONValue {
    fn into_json_value(self) -> serde_json::Value;
}
impl IntoJSONValue for rusqlite::types::Value {
    fn into_json_value(self) -> serde_json::Value {
        match self {
            rusqlite::types::Value::Null => serde_json::Value::Null,
            rusqlite::types::Value::Integer(value) => value.into(),
            rusqlite::types::Value::Real(value) => value.into(),
            rusqlite::types::Value::Text(value) => value.into(),
            rusqlite::types::Value::Blob(value) => value.into(),
        }
    }
}
