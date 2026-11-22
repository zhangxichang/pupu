use eyre::{Result, eyre};
use sea_query::{Expr, ExprTrait, Iden, OnConflict, Order, Query, SqliteQueryBuilder, Value};

use crate::{SqlValue, Statement, StorageBackend};

#[derive(Debug, Clone)]
pub struct SyncCursor {
    pub conversation_id: String,
    pub peer_endpoint_id: Vec<u8>,
    pub sender_endpoint_id: Vec<u8>,
    pub last_seq: i64,
}

#[derive(Iden)]
#[iden = "conversation_sync"]
enum ConversationSyncTable {
    Table,
    ConversationId,
    PeerEndpointId,
    SenderEndpointId,
    LastSeq,
}

pub async fn get(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    peer_endpoint_id: &[u8],
    sender_endpoint_id: &[u8],
) -> Result<i64> {
    let (sql, values) = Query::select()
        .column(ConversationSyncTable::LastSeq)
        .from(ConversationSyncTable::Table)
        .and_where(Expr::col(ConversationSyncTable::ConversationId).eq(conversation_id))
        .and_where(Expr::col(ConversationSyncTable::PeerEndpointId).eq(peer_endpoint_id))
        .and_where(Expr::col(ConversationSyncTable::SenderEndpointId).eq(sender_endpoint_id))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            _ => Err(eyre!("同步游标字段不正确")),
        },
        None => Ok(0),
    }
}

pub async fn set(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    peer_endpoint_id: &[u8],
    sender_endpoint_id: &[u8],
    last_seq: i64,
) -> Result<()> {
    let (sql, values) = Query::insert()
        .into_table(ConversationSyncTable::Table)
        .columns([
            ConversationSyncTable::ConversationId,
            ConversationSyncTable::PeerEndpointId,
            ConversationSyncTable::SenderEndpointId,
            ConversationSyncTable::LastSeq,
        ])
        .values_panic([
            Value::String(Some(conversation_id.to_string())).into(),
            Value::Bytes(Some(peer_endpoint_id.to_vec())).into(),
            Value::Bytes(Some(sender_endpoint_id.to_vec())).into(),
            Value::BigInt(Some(last_seq)).into(),
        ])
        .on_conflict(
            OnConflict::columns([
                ConversationSyncTable::ConversationId,
                ConversationSyncTable::PeerEndpointId,
                ConversationSyncTable::SenderEndpointId,
            ])
            .update_columns([ConversationSyncTable::LastSeq])
            .to_owned(),
        )
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn cursors(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    peer_endpoint_id: &[u8],
) -> Result<Vec<SyncCursor>> {
    let (sql, values) = Query::select()
        .column(ConversationSyncTable::ConversationId)
        .column(ConversationSyncTable::PeerEndpointId)
        .column(ConversationSyncTable::SenderEndpointId)
        .column(ConversationSyncTable::LastSeq)
        .from(ConversationSyncTable::Table)
        .and_where(Expr::col(ConversationSyncTable::ConversationId).eq(conversation_id))
        .and_where(Expr::col(ConversationSyncTable::PeerEndpointId).eq(peer_endpoint_id))
        .order_by(ConversationSyncTable::SenderEndpointId, Order::Asc)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result.rows.into_iter().map(row_to_cursor).collect()
}

fn row_to_cursor(row: Vec<SqlValue>) -> Result<SyncCursor> {
    let mut row = row.into_iter();
    let conversation_id = text(row.next())?;
    let peer_endpoint_id = blob(row.next())?;
    let sender_endpoint_id = blob(row.next())?;
    let last_seq = match row.next() {
        Some(SqlValue::Integer(value)) => value,
        _ => return Err(eyre!("同步游标字段不正确")),
    };
    Ok(SyncCursor {
        conversation_id,
        peer_endpoint_id,
        sender_endpoint_id,
        last_seq,
    })
}

fn text(value: Option<SqlValue>) -> Result<String> {
    match value {
        Some(SqlValue::Text(value)) => Ok(value),
        _ => Err(eyre!("同步文本字段不正确")),
    }
}

fn blob(value: Option<SqlValue>) -> Result<Vec<u8>> {
    match value {
        Some(SqlValue::Blob(value)) => Ok(value),
        _ => Err(eyre!("同步二进制字段不正确")),
    }
}
