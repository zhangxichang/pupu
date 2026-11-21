use eyre::{Result, eyre};
use sea_query::{
    Expr, ExprTrait, Iden, OnConflict, Order, Query, SqliteQueryBuilder, Value, Values,
};
use uuid::Uuid;

use crate::{SqlValue, Statement, StorageBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationKind {
    Private,
    Group,
}

impl ConversationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Group => "group",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "private" => Ok(Self::Private),
            "group" => Ok(Self::Group),
            _ => Err(eyre!("未知的会话类型：{value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationSummary {
    pub id: String,
    pub kind: ConversationKind,
    pub name: String,
    pub avatar: Option<Vec<u8>>,
    pub latest_message_id: Option<i64>,
    pub latest_markdown: Option<String>,
    pub latest_sender_id: Option<String>,
    pub latest_sender_name: Option<String>,
    pub latest_at: i64,
    pub unread_count: i64,
    pub network_id: Option<String>,
    pub topic_id: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub profile_id: String,
    pub name: String,
    pub avatar: Option<Vec<u8>>,
    pub last_read_message_id: Option<i64>,
    pub last_read_message_uid: Option<crate::messages::MessageUid>,
}

#[derive(Iden)]
#[iden = "conversation"]
enum ConversationTable {
    Table,
    Id,
    OwnerIdentity,
    Type,
    Name,
    AvatarHash,
    NetworkId,
    TopicId,
}

#[derive(Iden)]
#[iden = "conversation_member"]
enum ConversationMemberTable {
    Table,
    ConversationId,
    MemberId,
    LastReadMessageId,
    LastReadMessageUid,
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
}

#[derive(Iden)]
#[iden = "user_profile"]
enum UserProfileTable {
    Table,
    Id,
    Name,
    AvatarHash,
    EndpointId,
}

#[derive(Iden)]
#[iden = "attachment"]
enum AttachmentTable {
    Table,
    Hash,
    Data,
}

struct LatestMessage {
    id: i64,
    sender_id: String,
    sender_name: Option<String>,
    markdown: String,
    timestamp: i64,
}

pub async fn list(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    kind: ConversationKind,
) -> Result<Vec<ConversationSummary>> {
    let (sql, values) = Query::select()
        .column((ConversationTable::Table, ConversationTable::Id))
        .column((ConversationTable::Table, ConversationTable::Type))
        .column((ConversationTable::Table, ConversationTable::Name))
        .column((AttachmentTable::Table, AttachmentTable::Data))
        .column((ConversationTable::Table, ConversationTable::NetworkId))
        .column((ConversationTable::Table, ConversationTable::TopicId))
        .from(ConversationTable::Table)
        .left_join(
            AttachmentTable::Table,
            Expr::col((AttachmentTable::Table, AttachmentTable::Hash))
                .equals((ConversationTable::Table, ConversationTable::AvatarHash)),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::OwnerIdentity)).eq(identity_id),
        )
        .and_where(Expr::col((ConversationTable::Table, ConversationTable::Type)).eq(kind.as_str()))
        .order_by_expr(Expr::cust("conversation.rowid"), Order::Asc)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    let rows = result.rows;

    let mut summaries = Vec::with_capacity(rows.len());
    for row in rows {
        let mut row = row.into_iter();
        let id = match row.next() {
            Some(SqlValue::Text(value)) => value,
            _ => return Err(eyre!("会话 id 类型不正确")),
        };
        let kind = match row.next() {
            Some(SqlValue::Text(value)) => ConversationKind::from_str(&value)?,
            _ => return Err(eyre!("会话类型字段不正确")),
        };
        let stored_name = match row.next() {
            Some(SqlValue::Null) => None,
            Some(SqlValue::Text(value)) => Some(value),
            _ => return Err(eyre!("会话名称字段不正确")),
        };
        let avatar = match row.next() {
            Some(SqlValue::Null) => None,
            Some(SqlValue::Blob(value)) => Some(value),
            _ => return Err(eyre!("会话头像字段不正确")),
        };
        let network_id = match row.next() {
            Some(SqlValue::Null) => None,
            Some(SqlValue::Text(value)) => Some(value),
            _ => return Err(eyre!("会话网络 id 字段不正确")),
        };
        let topic_id = match row.next() {
            Some(SqlValue::Null) => None,
            Some(SqlValue::Blob(value)) => Some(value),
            _ => return Err(eyre!("会话 topic 字段不正确")),
        };

        let (name, avatar) = match kind {
            ConversationKind::Private => {
                let members = load_members(&mut *storage, &id).await?;
                let peer = members
                    .iter()
                    .find(|member| member.profile_id != identity_id);
                match peer {
                    Some(peer) => (peer.name.clone(), peer.avatar.clone()),
                    None => (
                        stored_name.unwrap_or_else(|| "未命名会话".to_string()),
                        avatar,
                    ),
                }
            }
            ConversationKind::Group => (stored_name.unwrap_or_else(|| "群聊".to_string()), avatar),
        };

        let latest = latest_message(&mut *storage, &id).await?;
        let last_read = last_read_message_id(&mut *storage, &id, identity_id).await?;
        let unread_count = count_unread(&mut *storage, &id, identity_id, last_read).await?;

        summaries.push(ConversationSummary {
            id,
            kind,
            name,
            avatar,
            latest_message_id: latest.as_ref().map(|message| message.id),
            latest_markdown: latest.as_ref().map(|message| message.markdown.clone()),
            latest_sender_id: latest.as_ref().map(|message| message.sender_id.clone()),
            latest_sender_name: latest
                .as_ref()
                .and_then(|message| message.sender_name.clone()),
            latest_at: latest.as_ref().map_or(0, |message| message.timestamp),
            unread_count,
            network_id,
            topic_id,
        });
    }

    summaries.sort_by(|left, right| {
        right
            .latest_message_id
            .unwrap_or(0)
            .cmp(&left.latest_message_id.unwrap_or(0))
    });
    Ok(summaries)
}

pub async fn ensure_private(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    peer_profile_id: &str,
) -> Result<String> {
    if identity_id == peer_profile_id {
        return Err(eyre!("不能和自己创建私聊会话"));
    }

    let (sql, values) = Query::select()
        .column((ConversationTable::Table, ConversationTable::Id))
        .from(ConversationTable::Table)
        .inner_join(
            ConversationMemberTable::Table,
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .equals((ConversationTable::Table, ConversationTable::Id)),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::OwnerIdentity)).eq(identity_id),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::Type))
                .eq(ConversationKind::Private.as_str()),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            ))
            .eq(peer_profile_id),
        )
        .limit(1)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    if let Some(row) = result.rows.into_iter().next() {
        return match row.into_iter().next() {
            Some(SqlValue::Text(value)) => Ok(value),
            _ => Err(eyre!("会话 id 类型不正确")),
        };
    }

    let conversation_id = Uuid::new_v4().to_string();
    let (sql, values) = Query::insert()
        .into_table(ConversationTable::Table)
        .columns([
            ConversationTable::Id,
            ConversationTable::OwnerIdentity,
            ConversationTable::Type,
            ConversationTable::Name,
            ConversationTable::AvatarHash,
        ])
        .values_panic([
            Value::String(Some(conversation_id.clone())).into(),
            Value::String(Some(identity_id.to_string())).into(),
            Value::String(Some(ConversationKind::Private.as_str().to_string())).into(),
            Value::String(None).into(),
            Value::Bytes(None).into(),
        ])
        .build(SqliteQueryBuilder);
    let mut statements = vec![Statement::from((sql, values))];
    statements.push(member_statement(&conversation_id, identity_id));
    statements.push(member_statement(&conversation_id, peer_profile_id));
    storage.execute_batch(statements).await?;
    Ok(conversation_id)
}

pub async fn create_group(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    name: &str,
    avatar: Option<&[u8]>,
    member_ids: &[String],
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(eyre!("群名称不能为空"));
    }

    let conversation_id = Uuid::new_v4().to_string();
    let mut statements = Vec::with_capacity(4 + member_ids.len());
    let avatar_hash = if let Some(avatar) = avatar {
        let (hash, statement) =
            crate::attachments::insert_statement(crate::attachments::AttachmentKind::Image, avatar);
        statements.push(statement);
        Some(hash)
    } else {
        None
    };

    let (sql, values) = Query::insert()
        .into_table(ConversationTable::Table)
        .columns([
            ConversationTable::Id,
            ConversationTable::OwnerIdentity,
            ConversationTable::Type,
            ConversationTable::Name,
            ConversationTable::AvatarHash,
        ])
        .values_panic([
            Value::String(Some(conversation_id.clone())).into(),
            Value::String(Some(identity_id.to_string())).into(),
            Value::String(Some(ConversationKind::Group.as_str().to_string())).into(),
            Value::String(Some(name.to_string())).into(),
            Value::Bytes(avatar_hash).into(),
        ])
        .build(SqliteQueryBuilder);
    statements.push(Statement::from((sql, values)));

    let mut members = vec![identity_id.to_string()];
    for member in member_ids {
        if !members.contains(member) {
            members.push(member.clone());
        }
    }
    for member in &members {
        statements.push(member_statement(&conversation_id, member));
    }

    storage.execute_batch(statements).await?;
    Ok(conversation_id)
}

#[derive(Debug, Clone)]
pub struct InviteMember {
    pub endpoint_id: [u8; 32],
    pub name: String,
}

pub async fn create_group_from_invite(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    network_id: &str,
    name: &str,
    members: &[InviteMember],
) -> Result<String> {
    if let Some(conversation_id) =
        find_by_network_id(&mut *storage, identity_id, network_id).await?
    {
        if kind(&mut *storage, identity_id, &conversation_id).await? == ConversationKind::Group {
            return Ok(conversation_id);
        }
        return Err(eyre!("网络 id 已被私聊占用"));
    }
    let name = name.trim();
    if name.is_empty() {
        return Err(eyre!("群名称不能为空"));
    }
    let self_endpoint = self_endpoint(&mut *storage, identity_id).await?;
    let conversation_id = Uuid::new_v4().to_string();
    let mut statements = Vec::with_capacity(2 + members.len());
    let (sql, values) = Query::insert()
        .into_table(ConversationTable::Table)
        .columns([
            ConversationTable::Id,
            ConversationTable::OwnerIdentity,
            ConversationTable::Type,
            ConversationTable::Name,
            ConversationTable::NetworkId,
        ])
        .values_panic([
            Value::String(Some(conversation_id.clone())).into(),
            Value::String(Some(identity_id.to_string())).into(),
            Value::String(Some(ConversationKind::Group.as_str().to_string())).into(),
            Value::String(Some(name.to_string())).into(),
            Value::String(Some(network_id.to_string())).into(),
        ])
        .build(SqliteQueryBuilder);
    statements.push(Statement::from((sql, values)));
    statements.push(member_statement(&conversation_id, identity_id));

    for member in members {
        if Some(member.endpoint_id) == self_endpoint {
            continue;
        }
        let profile_id = match crate::contacts::find_by_endpoint(
            &mut *storage,
            identity_id,
            &member.endpoint_id,
        )
        .await?
        {
            Some(contact) => contact.profile_id,
            None => {
                let profile_id = Uuid::new_v4().to_string();
                let (sql, values) = Query::insert()
                    .into_table(UserProfileTable::Table)
                    .columns([
                        UserProfileTable::Id,
                        UserProfileTable::Name,
                        UserProfileTable::EndpointId,
                    ])
                    .values_panic([
                        Value::String(Some(profile_id.clone())).into(),
                        Value::String(Some(member.name.clone())).into(),
                        Value::Bytes(Some(member.endpoint_id.to_vec())).into(),
                    ])
                    .build(SqliteQueryBuilder);
                statements.push(Statement::from((sql, values)));
                profile_id
            }
        };
        statements.push(member_statement(&conversation_id, &profile_id));
    }
    storage.execute_batch(statements).await?;
    Ok(conversation_id)
}

async fn self_endpoint(
    storage: &mut impl StorageBackend,
    identity_id: &str,
) -> Result<Option<[u8; 32]>> {
    let result = storage
        .query(Statement::from((
            "SELECT endpoint_id FROM user_profile WHERE id = ?",
            Values(vec![Value::String(Some(identity_id.to_string()))]),
        )))
        .await?;
    let Some(row) = result.rows.into_iter().next() else {
        return Ok(None);
    };
    match row.into_iter().next() {
        Some(SqlValue::Blob(value)) => Ok(Some(
            value
                .try_into()
                .map_err(|_| eyre!("本机节点 ID 长度不正确"))?,
        )),
        Some(SqlValue::Null) | None => Ok(None),
        _ => Err(eyre!("本机节点 ID 字段不正确")),
    }
}

pub async fn kind(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
) -> Result<ConversationKind> {
    ensure_owned(storage, identity_id, conversation_id).await
}

#[derive(Debug, Clone)]
pub struct SyncTarget {
    pub conversation_id: String,
    pub network_id: String,
    pub peer_profile_id: String,
    pub kind: ConversationKind,
}

pub async fn sync_targets_for_endpoint(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    endpoint_id: &[u8],
) -> Result<Vec<SyncTarget>> {
    let (sql, values) = Query::select()
        .column((ConversationTable::Table, ConversationTable::Id))
        .column((ConversationTable::Table, ConversationTable::NetworkId))
        .column((UserProfileTable::Table, UserProfileTable::Id))
        .column((ConversationTable::Table, ConversationTable::Type))
        .from(ConversationTable::Table)
        .inner_join(
            ConversationMemberTable::Table,
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .equals((ConversationTable::Table, ConversationTable::Id)),
        )
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id)).equals((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            )),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::OwnerIdentity)).eq(identity_id),
        )
        .and_where(
            Expr::col((UserProfileTable::Table, UserProfileTable::EndpointId)).eq(endpoint_id),
        )
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    let mut targets = Vec::with_capacity(result.rows.len());
    for row in result.rows {
        let mut row = row.into_iter();
        let conversation_id = match row.next() {
            Some(SqlValue::Text(value)) => value,
            _ => continue,
        };
        let network_id = match row.next() {
            Some(SqlValue::Text(value)) => value,
            _ => continue,
        };
        let peer_profile_id = match row.next() {
            Some(SqlValue::Text(value)) => value,
            _ => continue,
        };
        let kind = match row.next() {
            Some(SqlValue::Text(value)) => ConversationKind::from_str(&value)?,
            _ => continue,
        };
        targets.push(SyncTarget {
            conversation_id,
            network_id,
            peer_profile_id,
            kind,
        });
    }
    Ok(targets)
}

#[derive(Debug, Clone)]
pub struct SenderSync {
    pub profile_id: String,
    pub endpoint_id: [u8; 32],
    pub last_seq: i64,
}

pub async fn sync_senders(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
) -> Result<Vec<SenderSync>> {
    let result = storage
        .query(Statement::from((
            "SELECT DISTINCT m.sender_id, p.endpoint_id              FROM message m JOIN user_profile p ON p.id = m.sender_id              WHERE m.owner_conversation = ?",
            Values(vec![Value::String(Some(conversation_id.to_string()))]),
        )))
        .await?;
    let mut senders = Vec::with_capacity(result.rows.len());
    for row in result.rows {
        let mut row = row.into_iter();
        let profile_id = match row.next() {
            Some(SqlValue::Text(value)) => value,
            _ => continue,
        };
        let endpoint_id = match row.next() {
            Some(SqlValue::Blob(value)) => match value.try_into() {
                Ok(endpoint_id) => endpoint_id,
                Err(_) => continue,
            },
            _ => continue,
        };
        let last_seq =
            crate::messages::contiguous_sender_seq(storage, conversation_id, &profile_id).await?;
        senders.push(SenderSync {
            profile_id,
            endpoint_id,
            last_seq,
        });
    }
    Ok(senders)
}

pub async fn member_endpoints(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
) -> Result<Vec<[u8; 32]>> {
    let (sql, values) = Query::select()
        .column((UserProfileTable::Table, UserProfileTable::EndpointId))
        .from(ConversationMemberTable::Table)
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id)).equals((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            )),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .eq(conversation_id),
        )
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    let mut endpoints = Vec::with_capacity(result.rows.len());
    for row in result.rows {
        if let Some(SqlValue::Blob(value)) = row.into_iter().next()
            && let Ok(endpoint) = value.try_into()
        {
            endpoints.push(endpoint);
        }
    }
    Ok(endpoints)
}

pub async fn member_profile_by_endpoint(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    endpoint_id: &[u8],
) -> Result<Option<String>> {
    let (sql, values) = Query::select()
        .column((UserProfileTable::Table, UserProfileTable::Id))
        .from(ConversationMemberTable::Table)
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id)).equals((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            )),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .eq(conversation_id),
        )
        .and_where(
            Expr::col((UserProfileTable::Table, UserProfileTable::EndpointId)).eq(endpoint_id),
        )
        .limit(1)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Text(value)) => Ok(Some(value)),
            _ => Err(eyre!("群成员 id 字段不正确")),
        },
        None => Ok(None),
    }
}

pub async fn members(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
) -> Result<Vec<Member>> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    load_members(storage, conversation_id).await
}

pub async fn mark_read(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    message_id: Option<i64>,
) -> Result<()> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = Query::insert()
        .into_table(ConversationMemberTable::Table)
        .columns([
            ConversationMemberTable::ConversationId,
            ConversationMemberTable::MemberId,
            ConversationMemberTable::LastReadMessageId,
        ])
        .values_panic([
            Value::String(Some(conversation_id.to_string())).into(),
            Value::String(Some(identity_id.to_string())).into(),
            Value::BigInt(message_id).into(),
        ])
        .on_conflict(
            OnConflict::columns([
                ConversationMemberTable::ConversationId,
                ConversationMemberTable::MemberId,
            ])
            .update_columns([ConversationMemberTable::LastReadMessageId])
            .to_owned(),
        )
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn set_network_id(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    network_id: Option<&str>,
) -> Result<()> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = Query::update()
        .table(ConversationTable::Table)
        .value(
            ConversationTable::NetworkId,
            Value::String(network_id.map(str::to_string)),
        )
        .and_where(Expr::col(ConversationTable::Id).eq(conversation_id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn set_topic_id(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    topic_id: Option<&[u8]>,
) -> Result<()> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = Query::update()
        .table(ConversationTable::Table)
        .value(
            ConversationTable::TopicId,
            Value::Bytes(topic_id.map(<[u8]>::to_vec)),
        )
        .and_where(Expr::col(ConversationTable::Id).eq(conversation_id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn network_id(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
) -> Result<Option<String>> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = Query::select()
        .column(ConversationTable::NetworkId)
        .from(ConversationTable::Table)
        .and_where(Expr::col(ConversationTable::Id).eq(conversation_id))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Null) | None => Ok(None),
            Some(SqlValue::Text(value)) => Ok(Some(value)),
            _ => Err(eyre!("会话网络 id 字段不正确")),
        },
        None => Ok(None),
    }
}

pub async fn peer_endpoint(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
) -> Result<Option<[u8; 32]>> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = Query::select()
        .column((UserProfileTable::Table, UserProfileTable::EndpointId))
        .from(ConversationMemberTable::Table)
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id)).equals((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            )),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .eq(conversation_id),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            ))
            .ne(identity_id),
        )
        .limit(1)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Blob(value)) => Ok(Some(
                value
                    .try_into()
                    .map_err(|_| eyre!("联系人节点 ID 长度不正确"))?,
            )),
            Some(SqlValue::Null) | None => Ok(None),
            _ => Err(eyre!("联系人节点 ID 字段不正确")),
        },
        None => Ok(None),
    }
}

pub async fn find_private_by_endpoint(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    endpoint_id: &[u8],
) -> Result<Option<String>> {
    let (sql, values) = Query::select()
        .column((ConversationTable::Table, ConversationTable::Id))
        .from(ConversationTable::Table)
        .inner_join(
            ConversationMemberTable::Table,
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .equals((ConversationTable::Table, ConversationTable::Id)),
        )
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id)).equals((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            )),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::OwnerIdentity)).eq(identity_id),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::Type))
                .eq(ConversationKind::Private.as_str()),
        )
        .and_where(
            Expr::col((UserProfileTable::Table, UserProfileTable::EndpointId)).eq(endpoint_id),
        )
        .limit(1)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Text(value)) => Ok(Some(value)),
            _ => Err(eyre!("会话 id 类型不正确")),
        },
        None => Ok(None),
    }
}

pub async fn find_by_network_id(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    network_id: &str,
) -> Result<Option<String>> {
    let (sql, values) = Query::select()
        .column((ConversationTable::Table, ConversationTable::Id))
        .from(ConversationTable::Table)
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::OwnerIdentity)).eq(identity_id),
        )
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::NetworkId)).eq(network_id),
        )
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Text(value)) => Ok(Some(value)),
            _ => Err(eyre!("会话 id 类型不正确")),
        },
        None => Ok(None),
    }
}

pub async fn mark_read_uid(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
    uid: &[u8; 16],
) -> Result<()> {
    ensure_owned(&mut *storage, identity_id, conversation_id).await?;
    let (sql, values) = Query::insert()
        .into_table(ConversationMemberTable::Table)
        .columns([
            ConversationMemberTable::ConversationId,
            ConversationMemberTable::MemberId,
            ConversationMemberTable::LastReadMessageUid,
        ])
        .values_panic([
            Value::String(Some(conversation_id.to_string())).into(),
            Value::String(Some(identity_id.to_string())).into(),
            Value::Bytes(Some(uid.to_vec())).into(),
        ])
        .on_conflict(
            OnConflict::columns([
                ConversationMemberTable::ConversationId,
                ConversationMemberTable::MemberId,
            ])
            .update_columns([ConversationMemberTable::LastReadMessageUid])
            .to_owned(),
        )
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn mark_member_read(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    member_id: &str,
    message_id: Option<i64>,
    uid: &[u8; 16],
) -> Result<()> {
    if let Some(new_id) = message_id {
        let (sql, values) = Query::select()
            .column(ConversationMemberTable::LastReadMessageId)
            .from(ConversationMemberTable::Table)
            .and_where(Expr::col(ConversationMemberTable::ConversationId).eq(conversation_id))
            .and_where(Expr::col(ConversationMemberTable::MemberId).eq(member_id))
            .build(SqliteQueryBuilder);
        let result = storage.query(Statement::from((sql, values))).await?;
        if let Some(Some(SqlValue::Integer(current))) = result
            .rows
            .into_iter()
            .next()
            .map(|row| row.into_iter().next())
            && current >= new_id
        {
            return Ok(());
        }
    }
    let (sql, values) = Query::insert()
        .into_table(ConversationMemberTable::Table)
        .columns([
            ConversationMemberTable::ConversationId,
            ConversationMemberTable::MemberId,
            ConversationMemberTable::LastReadMessageId,
            ConversationMemberTable::LastReadMessageUid,
        ])
        .values_panic([
            Value::String(Some(conversation_id.to_string())).into(),
            Value::String(Some(member_id.to_string())).into(),
            Value::BigInt(message_id).into(),
            Value::Bytes(Some(uid.to_vec())).into(),
        ])
        .on_conflict(
            OnConflict::columns([
                ConversationMemberTable::ConversationId,
                ConversationMemberTable::MemberId,
            ])
            .update_columns([
                ConversationMemberTable::LastReadMessageId,
                ConversationMemberTable::LastReadMessageUid,
            ])
            .to_owned(),
        )
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn mark_member_read_uid(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    member_id: &str,
    uid: &[u8; 16],
) -> Result<()> {
    let (sql, values) = Query::insert()
        .into_table(ConversationMemberTable::Table)
        .columns([
            ConversationMemberTable::ConversationId,
            ConversationMemberTable::MemberId,
            ConversationMemberTable::LastReadMessageUid,
        ])
        .values_panic([
            Value::String(Some(conversation_id.to_string())).into(),
            Value::String(Some(member_id.to_string())).into(),
            Value::Bytes(Some(uid.to_vec())).into(),
        ])
        .on_conflict(
            OnConflict::columns([
                ConversationMemberTable::ConversationId,
                ConversationMemberTable::MemberId,
            ])
            .update_columns([ConversationMemberTable::LastReadMessageUid])
            .to_owned(),
        )
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub(crate) async fn ensure_owned(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    conversation_id: &str,
) -> Result<ConversationKind> {
    let (sql, values) = Query::select()
        .column((ConversationTable::Table, ConversationTable::Type))
        .from(ConversationTable::Table)
        .and_where(Expr::col((ConversationTable::Table, ConversationTable::Id)).eq(conversation_id))
        .and_where(
            Expr::col((ConversationTable::Table, ConversationTable::OwnerIdentity)).eq(identity_id),
        )
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Text(value)) => ConversationKind::from_str(&value),
            _ => Err(eyre!("会话类型字段不正确")),
        },
        None => Err(eyre!("会话不存在或不属于当前账户")),
    }
}

fn member_statement(conversation_id: &str, profile_id: &str) -> Statement {
    let (sql, values) = Query::insert()
        .into_table(ConversationMemberTable::Table)
        .columns([
            ConversationMemberTable::ConversationId,
            ConversationMemberTable::MemberId,
        ])
        .values_panic([
            Value::String(Some(conversation_id.to_string())).into(),
            Value::String(Some(profile_id.to_string())).into(),
        ])
        .build(SqliteQueryBuilder);
    Statement::from((sql, values))
}

async fn load_members(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
) -> Result<Vec<Member>> {
    let (sql, values) = Query::select()
        .column((UserProfileTable::Table, UserProfileTable::Id))
        .column((UserProfileTable::Table, UserProfileTable::Name))
        .column((AttachmentTable::Table, AttachmentTable::Data))
        .column((
            ConversationMemberTable::Table,
            ConversationMemberTable::LastReadMessageId,
        ))
        .column((
            ConversationMemberTable::Table,
            ConversationMemberTable::LastReadMessageUid,
        ))
        .from(ConversationMemberTable::Table)
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id)).equals((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            )),
        )
        .left_join(
            AttachmentTable::Table,
            Expr::col((AttachmentTable::Table, AttachmentTable::Hash))
                .equals((UserProfileTable::Table, UserProfileTable::AvatarHash)),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .eq(conversation_id),
        )
        .order_by_expr(Expr::cust("conversation_member.rowid"), Order::Asc)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result
        .rows
        .into_iter()
        .map(|row| {
            let mut row = row.into_iter();
            let profile_id = match row.next() {
                Some(SqlValue::Text(value)) => value,
                _ => return Err(eyre!("成员 id 类型不正确")),
            };
            let name = match row.next() {
                Some(SqlValue::Text(value)) => value,
                Some(SqlValue::Null) => "未知成员".to_string(),
                _ => return Err(eyre!("成员姓名字段不正确")),
            };
            let avatar = match row.next() {
                Some(SqlValue::Null) => None,
                Some(SqlValue::Blob(value)) => Some(value),
                _ => return Err(eyre!("成员头像字段不正确")),
            };
            let last_read_message_id = match row.next() {
                Some(SqlValue::Integer(value)) => Some(value),
                Some(SqlValue::Null) | None => None,
                _ => return Err(eyre!("成员最后读取位置字段不正确")),
            };
            let last_read_message_uid = match row.next() {
                Some(SqlValue::Blob(value)) => Some(
                    value
                        .try_into()
                        .map_err(|_| eyre!("成员最后读取消息 uid 长度不正确"))?,
                ),
                Some(SqlValue::Null) | None => None,
                _ => return Err(eyre!("成员最后读取消息 uid 字段不正确")),
            };
            Ok(Member {
                profile_id,
                name,
                avatar,
                last_read_message_id,
                last_read_message_uid,
            })
        })
        .collect()
}

async fn latest_message(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
) -> Result<Option<LatestMessage>> {
    let (sql, values) = Query::select()
        .column((MessageTable::Table, MessageTable::Id))
        .column((MessageTable::Table, MessageTable::SenderId))
        .column((MessageTable::Table, MessageTable::Markdown))
        .column((MessageTable::Table, MessageTable::Timestamp))
        .column((UserProfileTable::Table, UserProfileTable::Name))
        .from(MessageTable::Table)
        .left_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id))
                .equals((MessageTable::Table, MessageTable::SenderId)),
        )
        .and_where(
            Expr::col((MessageTable::Table, MessageTable::OwnerConversation)).eq(conversation_id),
        )
        .order_by((MessageTable::Table, MessageTable::Id), Order::Desc)
        .limit(1)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result
        .rows
        .into_iter()
        .next()
        .map(|row| {
            let mut row = row.into_iter();
            let id = match row.next() {
                Some(SqlValue::Integer(value)) => value,
                _ => return Err(eyre!("消息 id 类型不正确")),
            };
            let sender_id = match row.next() {
                Some(SqlValue::Text(value)) => value,
                _ => return Err(eyre!("消息发送者字段不正确")),
            };
            let markdown = match row.next() {
                Some(SqlValue::Text(value)) => value,
                _ => return Err(eyre!("消息内容字段不正确")),
            };
            let timestamp = match row.next() {
                Some(SqlValue::Integer(value)) => value,
                _ => return Err(eyre!("消息时间字段不正确")),
            };
            let sender_name = match row.next() {
                Some(SqlValue::Text(value)) => Some(value),
                Some(SqlValue::Null) => None,
                _ => return Err(eyre!("消息发送者姓名字段不正确")),
            };
            Ok(LatestMessage {
                id,
                sender_id,
                sender_name,
                markdown,
                timestamp,
            })
        })
        .transpose()
}

async fn last_read_message_id(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    identity_id: &str,
) -> Result<i64> {
    let (sql, values) = Query::select()
        .column((
            ConversationMemberTable::Table,
            ConversationMemberTable::LastReadMessageId,
        ))
        .column((
            ConversationMemberTable::Table,
            ConversationMemberTable::LastReadMessageUid,
        ))
        .from(ConversationMemberTable::Table)
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::ConversationId,
            ))
            .eq(conversation_id),
        )
        .and_where(
            Expr::col((
                ConversationMemberTable::Table,
                ConversationMemberTable::MemberId,
            ))
            .eq(identity_id),
        )
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            Some(SqlValue::Null) | None => Ok(0),
            _ => Err(eyre!("最后读取消息字段不正确")),
        },
        None => Ok(0),
    }
}

async fn count_unread(
    storage: &mut impl StorageBackend,
    conversation_id: &str,
    identity_id: &str,
    last_read_message_id: i64,
) -> Result<i64> {
    let result = storage
        .query(Statement::from((
            "SELECT count(*) FROM message \
             WHERE owner_conversation = ? AND id > ? AND sender_id != ?",
            Values(vec![
                Value::String(Some(conversation_id.to_string())),
                Value::BigInt(Some(last_read_message_id)),
                Value::String(Some(identity_id.to_string())),
            ]),
        )))
        .await?;
    match result.rows.into_iter().next() {
        Some(row) => match row.into_iter().next() {
            Some(SqlValue::Integer(value)) => Ok(value),
            _ => Err(eyre!("未读数字段不正确")),
        },
        None => Ok(0),
    }
}
