use eyre::{Result, eyre};
use sea_query::{Expr, ExprTrait, Iden, Order, Query, SqliteQueryBuilder, Value};
use uuid::Uuid;

use crate::{SqlValue, Statement, StorageBackend};

#[derive(Debug, Clone)]
pub struct Contact {
    pub profile_id: String,
    pub name: String,
    pub bio: Option<String>,
    pub avatar: Option<Vec<u8>>,
    pub endpoint_id: Option<Vec<u8>>,
    pub state: ContactState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContactState {
    PendingOut,
    PendingIn,
    #[default]
    Accepted,
    Rejected,
}

impl ContactState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingOut => "pending_out",
            Self::PendingIn => "pending_in",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending_out" => Some(Self::PendingOut),
            "pending_in" => Some(Self::PendingIn),
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

pub struct NewContact<'a> {
    pub name: &'a str,
    pub bio: Option<&'a str>,
    pub avatar: Option<&'a [u8]>,
    pub endpoint_id: Option<&'a [u8]>,
    pub state: ContactState,
}

pub struct RemoteProfile<'a> {
    pub name: &'a str,
    pub bio: Option<&'a str>,
    pub endpoint_id: &'a [u8],
}

#[derive(Iden)]
#[iden = "contact"]
enum ContactTable {
    Table,
    IdentityId,
    ProfileId,
    State,
}

#[derive(Iden)]
#[iden = "user_profile"]
enum UserProfileTable {
    Table,
    Id,
    Name,
    AvatarHash,
    Bio,
    EndpointId,
}

#[derive(Iden)]
#[iden = "attachment"]
enum AttachmentTable {
    Table,
    Hash,
    Data,
}

pub async fn list(storage: &mut impl StorageBackend, identity_id: &str) -> Result<Vec<Contact>> {
    let (sql, values) = Query::select()
        .column((UserProfileTable::Table, UserProfileTable::Id))
        .column((UserProfileTable::Table, UserProfileTable::Name))
        .column((UserProfileTable::Table, UserProfileTable::Bio))
        .column((AttachmentTable::Table, AttachmentTable::Data))
        .column((UserProfileTable::Table, UserProfileTable::EndpointId))
        .column((ContactTable::Table, ContactTable::State))
        .from(ContactTable::Table)
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id))
                .equals((ContactTable::Table, ContactTable::ProfileId)),
        )
        .left_join(
            AttachmentTable::Table,
            Expr::col((AttachmentTable::Table, AttachmentTable::Hash))
                .equals((UserProfileTable::Table, UserProfileTable::AvatarHash)),
        )
        .and_where(Expr::col((ContactTable::Table, ContactTable::IdentityId)).eq(identity_id))
        .order_by_expr(Expr::cust("contact.rowid"), Order::Asc)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result.rows.into_iter().map(row_to_contact).collect()
}

pub async fn create(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    contact: NewContact<'_>,
) -> Result<Contact> {
    let name = contact.name.trim();
    if name.is_empty() {
        return Err(eyre!("联系人姓名不能为空"));
    }
    let profile_id = Uuid::new_v4().to_string();
    let mut statements = Vec::with_capacity(3);

    let avatar_hash = if let Some(avatar) = contact.avatar {
        let (hash, statement) =
            crate::attachments::insert_statement(crate::attachments::AttachmentKind::Image, avatar);
        statements.push(statement);
        Some(hash)
    } else {
        None
    };

    let (sql, values) = Query::insert()
        .into_table(UserProfileTable::Table)
        .columns([
            UserProfileTable::Id,
            UserProfileTable::Name,
            UserProfileTable::AvatarHash,
            UserProfileTable::Bio,
            UserProfileTable::EndpointId,
        ])
        .values_panic([
            Value::String(Some(profile_id.clone())).into(),
            Value::String(Some(name.to_string())).into(),
            Value::Bytes(avatar_hash.clone()).into(),
            Value::String(contact.bio.map(str::to_string)).into(),
            Value::Bytes(contact.endpoint_id.map(<[u8]>::to_vec)).into(),
        ])
        .build(SqliteQueryBuilder);
    statements.push(Statement::from((sql, values)));

    let (sql, values) = Query::insert()
        .into_table(ContactTable::Table)
        .columns([
            ContactTable::IdentityId,
            ContactTable::ProfileId,
            ContactTable::State,
        ])
        .values_panic([
            Value::String(Some(identity_id.to_string())).into(),
            Value::String(Some(profile_id.clone())).into(),
            Value::String(Some(contact.state.as_str().to_string())).into(),
        ])
        .build(SqliteQueryBuilder);
    statements.push(Statement::from((sql, values)));

    storage.execute_batch(statements).await?;

    Ok(Contact {
        profile_id,
        name: name.to_string(),
        bio: contact.bio.map(str::to_string),
        avatar: contact.avatar.map(<[u8]>::to_vec),
        endpoint_id: contact.endpoint_id.map(<[u8]>::to_vec),
        state: contact.state,
    })
}

pub async fn remove(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    profile_id: &str,
) -> Result<()> {
    let (sql, values) = Query::delete()
        .from_table(ContactTable::Table)
        .and_where(Expr::col(ContactTable::IdentityId).eq(identity_id))
        .and_where(Expr::col(ContactTable::ProfileId).eq(profile_id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn upsert_remote(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    profile: RemoteProfile<'_>,
    state: ContactState,
) -> Result<Contact> {
    if profile.endpoint_id.len() != 32 {
        return Err(eyre!("节点 ID 长度必须为 32 字节"));
    }
    let name = profile.name.trim();
    let name = if name.is_empty() {
        "未命名联系人"
    } else {
        name
    };
    if let Some(existing) =
        find_by_endpoint(&mut *storage, identity_id, profile.endpoint_id).await?
    {
        if state == ContactState::PendingIn && existing.state == ContactState::Accepted {
            return Ok(existing);
        }
        let (sql, values) = Query::update()
            .table(UserProfileTable::Table)
            .value(
                UserProfileTable::Name,
                Value::String(Some(name.to_string())),
            )
            .value(
                UserProfileTable::Bio,
                Value::String(profile.bio.map(str::to_string)),
            )
            .and_where(Expr::col(UserProfileTable::Id).eq(&existing.profile_id))
            .build(SqliteQueryBuilder);
        storage.execute(Statement::from((sql, values))).await?;
        set_state(&mut *storage, identity_id, &existing.profile_id, state).await?;
        return find_by_endpoint(&mut *storage, identity_id, profile.endpoint_id)
            .await?
            .ok_or_else(|| eyre!("更新联系人后读取失败"));
    }

    let profile_id = Uuid::new_v4().to_string();
    let (sql, values) = Query::insert()
        .into_table(UserProfileTable::Table)
        .columns([
            UserProfileTable::Id,
            UserProfileTable::Name,
            UserProfileTable::AvatarHash,
            UserProfileTable::Bio,
            UserProfileTable::EndpointId,
        ])
        .values_panic([
            Value::String(Some(profile_id.clone())).into(),
            Value::String(Some(name.to_string())).into(),
            Value::Bytes(None).into(),
            Value::String(profile.bio.map(str::to_string)).into(),
            Value::Bytes(Some(profile.endpoint_id.to_vec())).into(),
        ])
        .build(SqliteQueryBuilder);
    let profile_statement = Statement::from((sql, values));
    let (sql, values) = Query::insert()
        .into_table(ContactTable::Table)
        .columns([
            ContactTable::IdentityId,
            ContactTable::ProfileId,
            ContactTable::State,
        ])
        .values_panic([
            Value::String(Some(identity_id.to_string())).into(),
            Value::String(Some(profile_id.clone())).into(),
            Value::String(Some(state.as_str().to_string())).into(),
        ])
        .build(SqliteQueryBuilder);
    storage
        .execute_batch(vec![profile_statement, Statement::from((sql, values))])
        .await?;
    find_by_endpoint(&mut *storage, identity_id, profile.endpoint_id)
        .await?
        .ok_or_else(|| eyre!("写入联系人后读取失败"))
}

pub async fn set_state(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    profile_id: &str,
    state: ContactState,
) -> Result<()> {
    let (sql, values) = Query::update()
        .table(ContactTable::Table)
        .value(
            ContactTable::State,
            Value::String(Some(state.as_str().to_string())),
        )
        .and_where(Expr::col(ContactTable::IdentityId).eq(identity_id))
        .and_where(Expr::col(ContactTable::ProfileId).eq(profile_id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn find_by_endpoint(
    storage: &mut impl StorageBackend,
    identity_id: &str,
    endpoint_id: &[u8],
) -> Result<Option<Contact>> {
    let (sql, values) = Query::select()
        .column((UserProfileTable::Table, UserProfileTable::Id))
        .column((UserProfileTable::Table, UserProfileTable::Name))
        .column((UserProfileTable::Table, UserProfileTable::Bio))
        .column((AttachmentTable::Table, AttachmentTable::Data))
        .column((UserProfileTable::Table, UserProfileTable::EndpointId))
        .column((ContactTable::Table, ContactTable::State))
        .from(ContactTable::Table)
        .inner_join(
            UserProfileTable::Table,
            Expr::col((UserProfileTable::Table, UserProfileTable::Id))
                .equals((ContactTable::Table, ContactTable::ProfileId)),
        )
        .left_join(
            AttachmentTable::Table,
            Expr::col((AttachmentTable::Table, AttachmentTable::Hash))
                .equals((UserProfileTable::Table, UserProfileTable::AvatarHash)),
        )
        .and_where(Expr::col((ContactTable::Table, ContactTable::IdentityId)).eq(identity_id))
        .and_where(
            Expr::col((UserProfileTable::Table, UserProfileTable::EndpointId)).eq(endpoint_id),
        )
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result
        .rows
        .into_iter()
        .next()
        .map(row_to_contact)
        .transpose()
}

fn row_to_contact(row: Vec<SqlValue>) -> Result<Contact> {
    let mut row = row.into_iter();
    let profile_id = match row.next() {
        Some(SqlValue::Text(value)) => value,
        _ => return Err(eyre!("联系人 id 类型不正确")),
    };
    let name = match row.next() {
        Some(SqlValue::Text(value)) => value,
        Some(SqlValue::Null) => "未命名联系人".to_string(),
        _ => return Err(eyre!("联系人姓名字段不正确")),
    };
    let bio = match row.next() {
        Some(SqlValue::Null) => None,
        Some(SqlValue::Text(value)) => Some(value),
        _ => return Err(eyre!("联系人签名字段不正确")),
    };
    let avatar = match row.next() {
        Some(SqlValue::Null) => None,
        Some(SqlValue::Blob(value)) => Some(value),
        _ => return Err(eyre!("联系人头像字段不正确")),
    };
    let endpoint_id = match row.next() {
        Some(SqlValue::Null) => None,
        Some(SqlValue::Blob(value)) => Some(value),
        _ => return Err(eyre!("联系人节点字段不正确")),
    };
    let state = match row.next() {
        Some(SqlValue::Text(value)) => {
            ContactState::from_str(&value).ok_or_else(|| eyre!("联系人状态字段不正确"))?
        }
        _ => return Err(eyre!("联系人状态字段不正确")),
    };
    Ok(Contact {
        profile_id,
        name,
        bio,
        avatar,
        endpoint_id,
        state,
    })
}
