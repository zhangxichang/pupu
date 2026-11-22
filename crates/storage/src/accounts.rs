use eyre::{Result, eyre};
use sea_query::{Expr, ExprTrait, Iden, OnConflict, Order, Query, SqliteQueryBuilder, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{SqlValue, Statement, StorageBackend};

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub avatar: Option<Vec<u8>>,
    pub endpoint_id: Option<Vec<u8>>,
}

#[derive(Iden)]
enum Identity {
    Table,
    Id,
    Key,
}

#[derive(Iden)]
enum UserProfile {
    Table,
    Id,
    Name,
    AvatarHash,
    Bio,
    EndpointId,
}

#[derive(Iden)]
enum Attachment {
    Table,
    Hash,
    Type,
    Data,
}

pub async fn list(storage: &mut impl StorageBackend) -> Result<Vec<Account>> {
    let (sql, values) = Query::select()
        .column((UserProfile::Table, UserProfile::Id))
        .column((UserProfile::Table, UserProfile::Name))
        .column((UserProfile::Table, UserProfile::Bio))
        .column((Attachment::Table, Attachment::Data))
        .column((UserProfile::Table, UserProfile::EndpointId))
        .from(Identity::Table)
        .inner_join(
            UserProfile::Table,
            Expr::col((UserProfile::Table, UserProfile::Id))
                .equals((Identity::Table, Identity::Id)),
        )
        .left_join(
            Attachment::Table,
            Expr::col((Attachment::Table, Attachment::Hash))
                .equals((UserProfile::Table, UserProfile::AvatarHash)),
        )
        .order_by_expr(Expr::cust("identity.rowid"), Order::Asc)
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result.rows.into_iter().map(row_to_account).collect()
}

pub async fn get(storage: &mut impl StorageBackend, id: &str) -> Result<Option<Account>> {
    let (sql, values) = Query::select()
        .column((UserProfile::Table, UserProfile::Id))
        .column((UserProfile::Table, UserProfile::Name))
        .column((UserProfile::Table, UserProfile::Bio))
        .column((Attachment::Table, Attachment::Data))
        .column((UserProfile::Table, UserProfile::EndpointId))
        .from(Identity::Table)
        .inner_join(
            UserProfile::Table,
            Expr::col((UserProfile::Table, UserProfile::Id))
                .equals((Identity::Table, Identity::Id)),
        )
        .left_join(
            Attachment::Table,
            Expr::col((Attachment::Table, Attachment::Hash))
                .equals((UserProfile::Table, UserProfile::AvatarHash)),
        )
        .and_where(Expr::col((UserProfile::Table, UserProfile::Id)).eq(id))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result
        .rows
        .into_iter()
        .next()
        .map(row_to_account)
        .transpose()
}

pub async fn create(
    storage: &mut impl StorageBackend,
    name: &str,
    avatar: Option<&[u8]>,
) -> Result<Account> {
    let name = name.trim();
    if name.is_empty() {
        return Err(eyre!("用户名不能为空"));
    }
    let id = Uuid::new_v4().to_string();
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).map_err(|err| eyre!("随机密钥生成失败：{err}"))?;
    let avatar_hash = avatar.map(|bytes| Sha256::digest(bytes).to_vec());
    let mut statements = Vec::with_capacity(3);

    if let Some(bytes) = avatar {
        let hash = avatar_hash
            .as_ref()
            .ok_or_else(|| eyre!("头像哈希生成失败"))?;
        let (sql, values) = Query::insert()
            .into_table(Attachment::Table)
            .columns([Attachment::Hash, Attachment::Type, Attachment::Data])
            .values_panic([
                Value::Bytes(Some(hash.clone())).into(),
                Value::String(Some("image".to_string())).into(),
                Value::Bytes(Some(bytes.to_vec())).into(),
            ])
            .on_conflict(OnConflict::column(Attachment::Hash).do_nothing().to_owned())
            .build(SqliteQueryBuilder);
        statements.push(Statement::from((sql, values)));
    }

    let (sql, values) = Query::insert()
        .into_table(UserProfile::Table)
        .columns([
            UserProfile::Id,
            UserProfile::Name,
            UserProfile::AvatarHash,
            UserProfile::Bio,
        ])
        .values_panic([
            Value::String(Some(id.clone())).into(),
            Value::String(Some(name.to_string())).into(),
            Value::Bytes(avatar_hash.clone()).into(),
            Value::String(None).into(),
        ])
        .build(SqliteQueryBuilder);
    statements.push(Statement::from((sql, values)));

    let (sql, values) = Query::insert()
        .into_table(Identity::Table)
        .columns([Identity::Id, Identity::Key])
        .values_panic([
            Value::String(Some(id.clone())).into(),
            Value::Bytes(Some(key.to_vec())).into(),
        ])
        .build(SqliteQueryBuilder);
    statements.push(Statement::from((sql, values)));

    storage.execute_batch(statements).await?;

    Ok(Account {
        id,
        name: name.to_string(),
        bio: None,
        avatar: avatar.map(|bytes| bytes.to_vec()),
        endpoint_id: None,
    })
}

pub async fn set_endpoint_id(
    storage: &mut impl StorageBackend,
    id: &str,
    endpoint_id: Option<&[u8]>,
) -> Result<()> {
    let (sql, values) = Query::update()
        .table(UserProfile::Table)
        .value(
            UserProfile::EndpointId,
            Value::Bytes(endpoint_id.map(<[u8]>::to_vec)),
        )
        .and_where(Expr::col(UserProfile::Id).eq(id))
        .build(SqliteQueryBuilder);
    storage.execute(Statement::from((sql, values))).await?;
    Ok(())
}

pub async fn identity_key(storage: &mut impl StorageBackend, id: &str) -> Result<Option<Vec<u8>>> {
    let (sql, values) = Query::select()
        .column(Identity::Key)
        .from(Identity::Table)
        .and_where(Expr::col(Identity::Id).eq(id))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    let Some(row) = result.rows.into_iter().next() else {
        return Ok(None);
    };
    match row.into_iter().next() {
        Some(SqlValue::Blob(value)) => Ok(Some(value)),
        _ => Err(eyre!("身份密钥字段类型不正确")),
    }
}

fn row_to_account(row: Vec<SqlValue>) -> Result<Account> {
    let mut row = row.into_iter();
    let id = text(row.next())?;
    let name = text(row.next())?;
    let bio = optional_text(row.next())?;
    let avatar = optional_blob(row.next())?;
    let endpoint_id = optional_blob(row.next())?;
    Ok(Account {
        id,
        name,
        bio,
        avatar,
        endpoint_id,
    })
}

fn text(value: Option<SqlValue>) -> Result<String> {
    match value {
        Some(SqlValue::Text(value)) => Ok(value),
        _ => Err(eyre!("账户字段类型不正确")),
    }
}

fn optional_text(value: Option<SqlValue>) -> Result<Option<String>> {
    match value {
        Some(SqlValue::Null) => Ok(None),
        Some(SqlValue::Text(value)) => Ok(Some(value)),
        _ => Err(eyre!("账户字段类型不正确")),
    }
}

fn optional_blob(value: Option<SqlValue>) -> Result<Option<Vec<u8>>> {
    match value {
        Some(SqlValue::Null) => Ok(None),
        Some(SqlValue::Blob(value)) => Ok(Some(value)),
        _ => Err(eyre!("头像字段类型不正确")),
    }
}
