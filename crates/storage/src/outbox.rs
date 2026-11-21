use eyre::{Result, eyre};
use sea_query::{Expr, ExprTrait, Iden, Order, Query, SqliteQueryBuilder, Value, Values};

use crate::{SqlValue, Statement, StorageBackend, messages::MessageUid};

#[derive(Debug, Clone)]
pub struct OutboxItem {
    pub id: i64,
    pub conversation_id: Option<String>,
    pub target_endpoint_id: Option<Vec<u8>>,
    pub message_uid: Option<MessageUid>,
    pub kind: String,
    pub payload: Vec<u8>,
    pub attempts: i64,
    pub next_attempt_at: i64,
    pub created_at: i64,
}

pub struct NewOutboxItem<'a> {
    pub conversation_id: Option<&'a str>,
    pub target_endpoint_id: Option<&'a [u8]>,
    pub message_uid: Option<&'a MessageUid>,
    pub kind: &'a str,
    pub payload: &'a [u8],
    pub created_at: i64,
    pub next_attempt_at: i64,
}

#[derive(Iden)]
#[iden = "outbox"]
enum OutboxTable {
    Table,
    Id,
    ConversationId,
    TargetEndpointId,
    MessageUid,
    Kind,
    Payload,
    Attempts,
    NextAttemptAt,
    CreatedAt,
}

pub async fn enqueue(storage: &mut impl StorageBackend, item: NewOutboxItem<'_>) -> Result<i64> {
    if item.kind.trim().is_empty() {
        return Err(eyre!("outbox 类型不能为空"));
    }
    let result = storage
        .execute_returning(Statement::from((
            "INSERT INTO outbox \
             (conversation_id, target_endpoint_id, message_uid, kind, payload, attempts, next_attempt_at, created_at) \
             VALUES (?, ?, ?, ?, ?, 0, ?, ?) RETURNING id",
            Values(vec![
                Value::String(item.conversation_id.map(str::to_string)),
                Value::Bytes(item.target_endpoint_id.map(<[u8]>::to_vec)),
                Value::Bytes(item.message_uid.map(|uid| uid.to_vec())),
                Value::String(Some(item.kind.to_string())),
                Value::Bytes(Some(item.payload.to_vec())),
                Value::BigInt(Some(item.next_attempt_at)),
                Value::BigInt(Some(item.created_at)),
            ]),
        )))
        .await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            _ => Err(eyre!("outbox id 类型不正确")),
        },
        None => Err(eyre!("outbox id 缺失")),
    }
}

pub async fn due(
    storage: &mut impl StorageBackend,
    now: i64,
    limit: i64,
) -> Result<Vec<OutboxItem>> {
    let (sql, values) = Query::select()
        .column(OutboxTable::Id)
        .column(OutboxTable::ConversationId)
        .column(OutboxTable::TargetEndpointId)
        .column(OutboxTable::MessageUid)
        .column(OutboxTable::Kind)
        .column(OutboxTable::Payload)
        .column(OutboxTable::Attempts)
        .column(OutboxTable::NextAttemptAt)
        .column(OutboxTable::CreatedAt)
        .from(OutboxTable::Table)
        .and_where(Expr::col(OutboxTable::NextAttemptAt).lte(now))
        .order_by(OutboxTable::NextAttemptAt, Order::Asc)
        .order_by(OutboxTable::Id, Order::Asc)
        .limit(if limit < 0 { 0 } else { limit as u64 })
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result.rows.into_iter().map(row_to_item).collect()
}

pub async fn record_attempt(
    storage: &mut impl StorageBackend,
    id: i64,
    attempts: i64,
    next_attempt_at: i64,
) -> Result<()> {
    let (sql, values) = Query::update()
        .table(OutboxTable::Table)
        .value(OutboxTable::Attempts, Value::BigInt(Some(attempts)))
        .value(
            OutboxTable::NextAttemptAt,
            Value::BigInt(Some(next_attempt_at)),
        )
        .and_where(Expr::col(OutboxTable::Id).eq(id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn remove(storage: &mut impl StorageBackend, id: i64) -> Result<()> {
    let (sql, values) = Query::delete()
        .from_table(OutboxTable::Table)
        .and_where(Expr::col(OutboxTable::Id).eq(id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn has_message_uid(storage: &mut impl StorageBackend, uid: &MessageUid) -> Result<bool> {
    let result = storage
        .query(Statement::from((
            "SELECT count(*) FROM outbox WHERE message_uid = ?",
            Values(vec![Value::Bytes(Some(uid.to_vec()))]),
        )))
        .await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value > 0),
            _ => Err(eyre!("outbox 消息数量字段不正确")),
        },
        None => Ok(false),
    }
}

pub async fn remove_message_target(
    storage: &mut impl StorageBackend,
    uid: &MessageUid,
    target_endpoint_id: &[u8],
) -> Result<()> {
    let (sql, values) = Query::delete()
        .from_table(OutboxTable::Table)
        .and_where(Expr::col(OutboxTable::MessageUid).eq(uid.as_slice()))
        .and_where(Expr::col(OutboxTable::TargetEndpointId).eq(target_endpoint_id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn count_for_message_uid(
    storage: &mut impl StorageBackend,
    uid: &MessageUid,
) -> Result<i64> {
    let result = storage
        .query(Statement::from((
            "SELECT count(*) FROM outbox WHERE message_uid = ?",
            Values(vec![Value::Bytes(Some(uid.to_vec()))]),
        )))
        .await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            _ => Err(eyre!("outbox 消息数量字段不正确")),
        },
        None => Ok(0),
    }
}

pub async fn count(storage: &mut impl StorageBackend) -> Result<i64> {
    let result = storage
        .query(Statement::from((
            "SELECT count(*) FROM outbox",
            Values(vec![]),
        )))
        .await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            _ => Err(eyre!("outbox 数量字段不正确")),
        },
        None => Ok(0),
    }
}

fn row_to_item(row: Vec<SqlValue>) -> Result<OutboxItem> {
    let mut row = row.into_iter();
    let id = integer(row.next())?;
    let conversation_id = optional_text(row.next())?;
    let target_endpoint_id = optional_blob(row.next())?;
    let message_uid = match row.next() {
        Some(SqlValue::Null) | None => None,
        Some(SqlValue::Blob(value)) => Some(
            value
                .try_into()
                .map_err(|_| eyre!("outbox 消息 uid 长度不正确"))?,
        ),
        _ => return Err(eyre!("outbox 消息 uid 字段不正确")),
    };
    let kind = text(row.next())?;
    let payload = blob(row.next())?;
    let attempts = integer(row.next())?;
    let next_attempt_at = integer(row.next())?;
    let created_at = integer(row.next())?;
    Ok(OutboxItem {
        id,
        conversation_id,
        target_endpoint_id,
        message_uid,
        kind,
        payload,
        attempts,
        next_attempt_at,
        created_at,
    })
}

fn integer(value: Option<SqlValue>) -> Result<i64> {
    match value {
        Some(SqlValue::Integer(value)) => Ok(value),
        _ => Err(eyre!("outbox 整数字段不正确")),
    }
}

fn text(value: Option<SqlValue>) -> Result<String> {
    match value {
        Some(SqlValue::Text(value)) => Ok(value),
        _ => Err(eyre!("outbox 文本字段不正确")),
    }
}

fn blob(value: Option<SqlValue>) -> Result<Vec<u8>> {
    match value {
        Some(SqlValue::Blob(value)) => Ok(value),
        _ => Err(eyre!("outbox 二进制字段不正确")),
    }
}

fn optional_text(value: Option<SqlValue>) -> Result<Option<String>> {
    match value {
        Some(SqlValue::Null) | None => Ok(None),
        Some(SqlValue::Text(value)) => Ok(Some(value)),
        _ => Err(eyre!("outbox 文本字段不正确")),
    }
}

fn optional_blob(value: Option<SqlValue>) -> Result<Option<Vec<u8>>> {
    match value {
        Some(SqlValue::Null) | None => Ok(None),
        Some(SqlValue::Blob(value)) => Ok(Some(value)),
        _ => Err(eyre!("outbox 二进制字段不正确")),
    }
}
