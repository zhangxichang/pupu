use eyre::{Result, eyre};
use sea_query::{Expr, ExprTrait, Iden, OnConflict, Query, SqliteQueryBuilder, Value};
use sha2::{Digest, Sha256};

use crate::{SqlValue, Statement, StorageBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    File,
    Image,
    Video,
    Audio,
}

impl AttachmentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "file" => Ok(Self::File),
            "image" => Ok(Self::Image),
            "video" => Ok(Self::Video),
            "audio" => Ok(Self::Audio),
            _ => Err(eyre!("未知的附件类型：{value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub hash: Vec<u8>,
    pub kind: AttachmentKind,
    pub data: Vec<u8>,
}

#[derive(Iden)]
#[iden = "attachment"]
enum AttachmentTable {
    Table,
    Hash,
    Type,
    Data,
    Blake3,
}

pub(crate) fn insert_statement(kind: AttachmentKind, data: &[u8]) -> (Vec<u8>, Statement) {
    let hash = Sha256::digest(data).to_vec();
    let blake3 = blake3_hash(data).to_vec();
    let (sql, values) = Query::insert()
        .into_table(AttachmentTable::Table)
        .columns([
            AttachmentTable::Hash,
            AttachmentTable::Type,
            AttachmentTable::Data,
            AttachmentTable::Blake3,
        ])
        .values_panic([
            Value::Bytes(Some(hash.clone())).into(),
            Value::String(Some(kind.as_str().to_string())).into(),
            Value::Bytes(Some(data.to_vec())).into(),
            Value::Bytes(Some(blake3)).into(),
        ])
        .on_conflict(
            OnConflict::column(AttachmentTable::Hash)
                .update_columns([AttachmentTable::Blake3])
                .to_owned(),
        )
        .build(SqliteQueryBuilder);
    (hash, Statement::from((sql, values)))
}

pub async fn put(
    storage: &mut impl StorageBackend,
    kind: AttachmentKind,
    data: &[u8],
) -> Result<Vec<u8>> {
    let (hash, statement) = insert_statement(kind, data);
    storage.execute(statement).await?;
    Ok(hash)
}

pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

pub async fn get_by_blake3(
    storage: &mut impl StorageBackend,
    blake3: &[u8],
) -> Result<Option<Attachment>> {
    let (sql, values) = Query::select()
        .column(AttachmentTable::Hash)
        .column(AttachmentTable::Type)
        .column(AttachmentTable::Data)
        .from(AttachmentTable::Table)
        .and_where(Expr::col(AttachmentTable::Blake3).eq(blake3))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result
        .rows
        .into_iter()
        .next()
        .map(row_to_attachment)
        .transpose()
}

pub async fn put_blob(
    storage: &mut impl StorageBackend,
    kind: AttachmentKind,
    data: &[u8],
) -> Result<([u8; 32], Vec<u8>)> {
    let (hash, statement) = insert_statement(kind, data);
    storage.execute(statement).await?;
    Ok((blake3_hash(data), hash))
}

pub async fn all_blobs(storage: &mut impl StorageBackend) -> Result<Vec<([u8; 32], Vec<u8>)>> {
    let result = storage
        .query(Statement::from((
            "SELECT blake3, data FROM attachment WHERE blake3 IS NOT NULL",
            sea_query::Values(vec![]),
        )))
        .await?;
    let mut blobs = Vec::with_capacity(result.rows.len());
    for row in result.rows {
        let mut row = row.into_iter();
        let hash = match row.next() {
            Some(SqlValue::Blob(value)) => value
                .try_into()
                .map_err(|_| eyre!("附件 BLAKE3 长度不正确"))?,
            _ => return Err(eyre!("附件 BLAKE3 字段不正确")),
        };
        let data = match row.next() {
            Some(SqlValue::Blob(value)) => value,
            _ => return Err(eyre!("附件数据字段不正确")),
        };
        blobs.push((hash, data));
    }
    Ok(blobs)
}

pub async fn get(storage: &mut impl StorageBackend, hash: &[u8]) -> Result<Option<Attachment>> {
    let (sql, values) = Query::select()
        .column(AttachmentTable::Hash)
        .column(AttachmentTable::Type)
        .column(AttachmentTable::Data)
        .from(AttachmentTable::Table)
        .and_where(Expr::col(AttachmentTable::Hash).eq(hash))
        .build(SqliteQueryBuilder);
    let result = storage.query(Statement::from((sql, values))).await?;
    result
        .rows
        .into_iter()
        .next()
        .map(row_to_attachment)
        .transpose()
}

#[derive(Debug, Clone)]
pub struct AttachmentMeta {
    pub kind: AttachmentKind,
    pub size: i64,
}

pub async fn get_metadata(
    storage: &mut impl StorageBackend,
    hash: &[u8],
) -> Result<Option<AttachmentMeta>> {
    let result = storage
        .query(Statement::from((
            "SELECT type, length(data) FROM attachment WHERE hash = ?",
            sea_query::Values(vec![Value::Bytes(Some(hash.to_vec()))]),
        )))
        .await?;
    let Some(row) = result.rows.into_iter().next() else {
        return Ok(None);
    };
    let mut row = row.into_iter();
    let kind = match row.next() {
        Some(SqlValue::Text(value)) => AttachmentKind::from_str(&value)?,
        _ => return Err(eyre!("附件类型字段不正确")),
    };
    let size = match row.next() {
        Some(SqlValue::Integer(value)) => value,
        _ => return Err(eyre!("附件大小字段不正确")),
    };
    Ok(Some(AttachmentMeta { kind, size }))
}

pub async fn get_metadata_by_blake3(
    storage: &mut impl StorageBackend,
    blake3: &[u8],
) -> Result<Option<AttachmentMeta>> {
    let result = storage
        .query(Statement::from((
            "SELECT type, length(data) FROM attachment WHERE blake3 = ?",
            sea_query::Values(vec![Value::Bytes(Some(blake3.to_vec()))]),
        )))
        .await?;
    let Some(row) = result.rows.into_iter().next() else {
        return Ok(None);
    };
    let mut row = row.into_iter();
    let kind = match row.next() {
        Some(SqlValue::Text(value)) => AttachmentKind::from_str(&value)?,
        _ => return Err(eyre!("附件类型字段不正确")),
    };
    let size = match row.next() {
        Some(SqlValue::Integer(value)) => value,
        _ => return Err(eyre!("附件大小字段不正确")),
    };
    Ok(Some(AttachmentMeta { kind, size }))
}

fn row_to_attachment(row: Vec<SqlValue>) -> Result<Attachment> {
    let mut row = row.into_iter();
    let hash = match row.next() {
        Some(SqlValue::Blob(value)) => value,
        _ => return Err(eyre!("附件 hash 类型不正确")),
    };
    let kind = match row.next() {
        Some(SqlValue::Text(value)) => AttachmentKind::from_str(&value)?,
        _ => return Err(eyre!("附件类型字段不正确")),
    };
    let data = match row.next() {
        Some(SqlValue::Blob(value)) => value,
        _ => return Err(eyre!("附件数据字段不正确")),
    };
    Ok(Attachment { hash, kind, data })
}
