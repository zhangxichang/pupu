use eyre::{Result, eyre};
use sea_query::{Expr, ExprTrait, Iden, Order, Query, SqliteQueryBuilder, Value, Values};

use crate::{SqlValue, Statement, StorageBackend};

pub const UID_LEN: usize = 16;
pub type MessageUid = [u8; UID_LEN];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Pending,
    Sent,
    Failed,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "sent" => Some(Self::Sent),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub uid: MessageUid,
    pub sender_seq: i64,
    pub status: MessageStatus,
    pub signature: Option<Vec<u8>>,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_avatar: Option<Vec<u8>>,
    pub markdown: String,
    pub timestamp: i64,
}

pub struct NewMessage<'a> {
    pub uid: MessageUid,
    pub sender_seq: i64,
    pub status: MessageStatus,
    pub signature: Option<&'a [u8]>,
    pub markdown: &'a str,
    pub timestamp: i64,
}

#[derive(Iden)]
#[iden = "message"]
enum MessageTable {
    Table,
    Id,
    OwnerConversation,
    SenderId,
    Markdown,
    Timestamp,
    Uid,
    SenderSeq,
    Status,
    Signature,
}

#[derive(Iden)]
#[iden = "user_profile"]
enum UserProfileTable {
    Table,
    Id,
    Name,
    AvatarHash,
}

#[derive(Iden)]
#[iden = "attachment"]
enum AttachmentTable {
    Table,
    Hash,
    Data,
}

pub async fn list(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
) -> Result<Vec<Message>> {
    crate::conversations::ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = message_query()
        .and_where(
            Expr::col((MessageTable::Table, MessageTable::OwnerConversation)).eq(conversation_id),
        )
        .order_by((MessageTable::Table, MessageTable::Id), Order::Asc)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result.rows.into_iter().map(row_to_message).collect()
}

pub async fn send_markdown(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    markdown: &str,
    timestamp: i64,
) -> Result<Message> {
    insert(
        storage,
        identity_id,
        conversation_id,
        identity_id,
        markdown,
        timestamp,
    )
    .await
}

pub async fn insert(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    sender_id: &str,
    markdown: &str,
    timestamp: i64,
) -> Result<Message> {
    crate::conversations::ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    ensure_sender_is_member(&mut *storage, conversation_id, sender_id).await?;
    if markdown.trim().is_empty() {
        return Err(eyre!("消息内容不能为空"));
    }

    let uid = random_uid()?;
    let sender_seq = next_sender_seq(&mut *storage, conversation_id, sender_id).await?;
    insert_rows(
        &mut *storage,
        identity_id,
        conversation_id,
        sender_id,
        NewMessage {
            uid,
            sender_seq,
            status: MessageStatus::Sent,
            signature: None,
            markdown,
            timestamp,
        },
    )
    .await
}

pub async fn prepare_outgoing(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    markdown: &str,
    timestamp: i64,
) -> Result<Message> {
    crate::conversations::ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    ensure_sender_is_member(&mut *storage, conversation_id, identity_id).await?;
    if markdown.trim().is_empty() {
        return Err(eyre!("消息内容不能为空"));
    }
    let uid = random_uid()?;
    let sender_seq = next_sender_seq(&mut *storage, conversation_id, identity_id).await?;
    insert_rows(
        &mut *storage,
        identity_id,
        conversation_id,
        identity_id,
        NewMessage {
            uid,
            sender_seq,
            status: MessageStatus::Pending,
            signature: None,
            markdown,
            timestamp,
        },
    )
    .await
}

pub async fn set_signature(
    storage: &mut impl StorageBackend,
    uid: &MessageUid,
    signature: &[u8],
) -> Result<()> {
    let (sql, values) = Query::update()
        .table(MessageTable::Table)
        .value(
            MessageTable::Signature,
            Value::Bytes(Some(signature.to_vec())),
        )
        .and_where(Expr::col(MessageTable::Uid).eq(uid.as_slice()))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn insert_network(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    sender_id: &str,
    message: NewMessage<'_>,
) -> Result<Message> {
    crate::conversations::ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    ensure_sender_is_member(&mut *storage, conversation_id, sender_id).await?;
    if message.markdown.trim().is_empty() {
        return Err(eyre!("消息内容不能为空"));
    }
    insert_rows(
        &mut *storage,
        identity_id,
        conversation_id,
        sender_id,
        message,
    )
    .await
}

async fn insert_rows(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    sender_id: &str,
    message: NewMessage<'_>,
) -> Result<Message> {
    let result = storage
        .execute_returning(Statement::from((
            "INSERT INTO message \
             (owner_conversation, sender_id, markdown, timestamp, uid, sender_seq, status, signature) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
            Values(vec![
                Value::String(Some(conversation_id.to_string())),
                Value::String(Some(sender_id.to_string())),
                Value::String(Some(message.markdown.to_string())),
                Value::BigInt(Some(message.timestamp)),
                Value::Bytes(Some(message.uid.to_vec())),
                Value::BigInt(Some(message.sender_seq)),
                Value::String(Some(message.status.as_str().to_string())),
                Value::Bytes(message.signature.map(<[u8]>::to_vec)),
            ]),
        )))
        .await?;
    let id = match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => value,
            _ => return Err(eyre!("新消息 id 类型不正确")),
        },
        None => return Err(eyre!("新消息 id 缺失")),
    };

    get_by_id(storage, identity_id, id)
        .await?
        .ok_or_else(|| eyre!("新消息写入后读取失败"))
}

pub async fn set_status(
    storage: &mut impl StorageBackend,
    uid: &MessageUid,
    status: MessageStatus,
) -> Result<()> {
    let (sql, values) = Query::update()
        .table(MessageTable::Table)
        .value(
            MessageTable::Status,
            Value::String(Some(status.as_str().to_string())),
        )
        .and_where(Expr::col(MessageTable::Uid).eq(uid.as_slice()))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn get_by_uid(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    uid: &MessageUid,
) -> Result<Option<Message>> {
    let (sql, values) = message_query()
        .and_where(Expr::col((MessageTable::Table, MessageTable::Uid)).eq(uid.as_slice()))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    let message = result
        .rows
        .into_iter()
        .next()
        .map(row_to_message)
        .transpose()?;
    if let Some(message) = &message
        && crate::conversations::ensure_owned(&mut *storage, identity_id, &message.conversation_id)
            .await
            .is_err()
    {
        return Ok(None);
    }
    Ok(message)
}

pub async fn after_seq(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    sender_id: &str,
    after_seq: i64,
    limit: i64,
) -> Result<Vec<Message>> {
    let (sql, values) = message_query()
        .and_where(
            Expr::col((MessageTable::Table, MessageTable::OwnerConversation)).eq(conversation_id),
        )
        .and_where(Expr::col((MessageTable::Table, MessageTable::SenderId)).eq(sender_id))
        .and_where(Expr::col((MessageTable::Table, MessageTable::SenderSeq)).gt(after_seq))
        .order_by((MessageTable::Table, MessageTable::SenderSeq), Order::Asc)
        .limit(if limit < 0 { 0 } else { limit as u64 })
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result.rows.into_iter().map(row_to_message).collect()
}

pub async fn pending_uids(storage: &mut impl StorageBackend) -> Result<Vec<MessageUid>> {
    let result = storage
        .query(Statement::from((
            "SELECT uid FROM message WHERE status = 'pending'",
            Values(vec![]),
        )))
        .await?;
    let mut uids = Vec::with_capacity(result.rows.len());
    for row in result.rows {
        if let Some(SqlValue::Blob(value)) = row.into_iter().next()
            && let Ok(uid) = value.try_into()
        {
            uids.push(uid);
        }
    }
    Ok(uids)
}

pub async fn contiguous_sender_seq(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    sender_id: &str,
) -> Result<i64> {
    let result = storage
        .query(Statement::from((
            "SELECT sender_seq FROM message              WHERE owner_conversation = ? AND sender_id = ? ORDER BY sender_seq ASC",
            Values(vec![
                Value::String(Some(conversation_id.to_string())),
                Value::String(Some(sender_id.to_string())),
            ]),
        )))
        .await?;
    let mut expected = 1i64;
    for row in result.rows {
        let seq = match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => value,
            _ => break,
        };
        if seq != expected {
            break;
        }
        expected += 1;
    }
    Ok(expected - 1)
}

pub async fn latest_uid(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
) -> Result<Option<MessageUid>> {
    let result = storage
        .query(Statement::from((
            "SELECT uid FROM message WHERE owner_conversation = ? ORDER BY id DESC LIMIT 1",
            Values(vec![Value::String(Some(conversation_id.to_string()))]),
        )))
        .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => return Ok(None),
    };
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Blob(value)) => Ok(Some(
                value.try_into().map_err(|_| eyre!("消息 uid 长度不正确"))?,
            )),
            _ => Ok(None),
        },
        None => Ok(None),
    }
}

pub async fn latest_sender_seq(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    sender_id: &str,
) -> Result<i64> {
    let result = storage
        .query(Statement::from((
            "SELECT COALESCE(MAX(sender_seq), 0) FROM message \
             WHERE owner_conversation = ? AND sender_id = ?",
            Values(vec![
                Value::String(Some(conversation_id.to_string())),
                Value::String(Some(sender_id.to_string())),
            ]),
        )))
        .await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            _ => Err(eyre!("消息序号字段不正确")),
        },
        None => Ok(0),
    }
}

async fn next_sender_seq(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    sender_id: &str,
) -> Result<i64> {
    Ok(latest_sender_seq(&mut *storage, conversation_id, sender_id).await? + 1)
}

fn message_query() -> sea_query::SelectStatement {
    Query::select()
        .column((MessageTable::Table, MessageTable::Id))
        .column((MessageTable::Table, MessageTable::OwnerConversation))
        .column((MessageTable::Table, MessageTable::SenderId))
        .column((UserProfileTable::Table, UserProfileTable::Name))
        .column((AttachmentTable::Table, AttachmentTable::Data))
        .column((MessageTable::Table, MessageTable::Markdown))
        .column((MessageTable::Table, MessageTable::Timestamp))
        .column((MessageTable::Table, MessageTable::Uid))
        .column((MessageTable::Table, MessageTable::SenderSeq))
        .column((MessageTable::Table, MessageTable::Status))
        .column((MessageTable::Table, MessageTable::Signature))
        .from(MessageTable::Table)
        .left_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id))
                .equals((MessageTable::Table, MessageTable::SenderId)),
        )
        .left_join(
            AttachmentTable::Table,
            Expr::col((AttachmentTable::Table, AttachmentTable::Hash))
                .equals((UserProfileTable::Table, UserProfileTable::AvatarHash)),
        )
        .to_owned()
}

async fn ensure_sender_is_member(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    sender_id: &str,
) -> Result<()> {
    let result = storage
        .query(Statement::from((
            "SELECT count(*) FROM conversation_member \
             WHERE conversation_id = ? AND member_id = ?",
            Values(vec![
                Value::String(Some(conversation_id.to_string())),
                Value::String(Some(sender_id.to_string())),
            ]),
        )))
        .await?;
    let count = match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => value,
            _ => return Err(eyre!("会话成员数量字段不正确")),
        },
        None => 0,
    };
    if count == 0 {
        return Err(eyre!("发送者不属于该会话"));
    }
    Ok(())
}

async fn get_by_id(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    id: i64,
) -> Result<Option<Message>> {
    let (sql, values) = message_query()
        .and_where(Expr::col((MessageTable::Table, MessageTable::Id)).eq(id))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    let message = result
        .rows
        .into_iter()
        .next()
        .map(row_to_message)
        .transpose()?;
    if let Some(message) = &message
        && crate::conversations::ensure_owned(&mut *storage, identity_id, &message.conversation_id)
            .await
            .is_err()
    {
        return Ok(None);
    }
    Ok(message)
}

fn row_to_message(row: Vec<SqlValue>) -> Result<Message> {
    let mut row = row.into_iter();
    let id = match row.next() {
        Some(SqlValue::Integer(value)) => value,
        _ => return Err(eyre!("消息 id 字段不正确")),
    };
    let conversation_id = match row.next() {
        Some(SqlValue::Text(value)) => value,
        _ => return Err(eyre!("消息会话字段不正确")),
    };
    let sender_id = match row.next() {
        Some(SqlValue::Text(value)) => value,
        _ => return Err(eyre!("消息发送者字段不正确")),
    };
    let sender_name = match row.next() {
        Some(SqlValue::Text(value)) => value,
        Some(SqlValue::Null) => "未知用户".to_string(),
        _ => return Err(eyre!("消息发送者姓名字段不正确")),
    };
    let sender_avatar = match row.next() {
        Some(SqlValue::Null) => None,
        Some(SqlValue::Blob(value)) => Some(value),
        _ => return Err(eyre!("消息发送者头像字段不正确")),
    };
    let markdown = match row.next() {
        Some(SqlValue::Text(value)) => value,
        _ => return Err(eyre!("消息内容字段不正确")),
    };
    let timestamp = match row.next() {
        Some(SqlValue::Integer(value)) => value,
        _ => return Err(eyre!("消息时间字段不正确")),
    };
    let uid = match row.next() {
        Some(SqlValue::Blob(value)) => {
            value.try_into().map_err(|_| eyre!("消息 uid 长度不正确"))?
        }
        _ => return Err(eyre!("消息 uid 字段不正确")),
    };
    let sender_seq = match row.next() {
        Some(SqlValue::Integer(value)) => value,
        _ => return Err(eyre!("消息序号字段不正确")),
    };
    let status = match row.next() {
        Some(SqlValue::Text(value)) => {
            MessageStatus::from_str(&value).ok_or_else(|| eyre!("消息状态字段不正确"))?
        }
        _ => return Err(eyre!("消息状态字段不正确")),
    };
    let signature = match row.next() {
        Some(SqlValue::Null) | None => None,
        Some(SqlValue::Blob(value)) => Some(value),
        _ => return Err(eyre!("消息签名字段不正确")),
    };
    Ok(Message {
        id,
        uid,
        sender_seq,
        status,
        signature,
        conversation_id,
        sender_id,
        sender_name,
        sender_avatar,
        markdown,
        timestamp,
    })
}

fn random_uid() -> Result<MessageUid> {
    let mut uid = [0u8; UID_LEN];
    getrandom::fill(&mut uid).map_err(|error| eyre!("消息 uid 生成失败：{error}"))?;
    Ok(uid)
}
