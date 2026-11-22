//! Wire format for the Pupu control protocol.
//!
//! Every message is a `u32` big-endian length followed by a postcard-encoded
//! payload. The length prefix is checked before any allocation so a peer can
//! never make us reserve an unbounded buffer.

use eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Current protocol version.
pub const PROTOCOL_VERSION: u16 = 1;
/// Hard upper bound for one control frame (1 MiB).
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

pub const ERROR_UNSUPPORTED_VERSION: u16 = 1000;
pub const ERROR_INVALID_REQUEST: u16 = 1001;
pub const ERROR_UNSUPPORTED: u16 = 1002;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub bio: Option<String>,
    pub relay_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub version: u16,
    pub profile: Profile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: u16,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SenderCursor {
    pub endpoint_id: [u8; 32],
    pub last_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub uid: [u8; 16],
    pub conversation_id: String,
    pub sender: [u8; 32],
    pub sender_seq: u64,
    pub timestamp: i64,
    pub markdown: String,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvite {
    pub group_id: String,
    pub network_id: String,
    pub name: String,
    pub members: Vec<[u8; 32]>,
    pub avatar_blake3: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    Hello(Hello),
    HelloAck(Hello),
    Ping,
    Pong,
    Error(ProtocolError),
    Ack {
        uid: [u8; 16],
    },
    ContactRequest {
        profile: Profile,
        message: String,
    },
    ContactAccept {
        profile: Profile,
    },
    ContactReject {
        reason: String,
    },
    Message(MessageEnvelope),
    SyncRequest {
        conversation_id: String,
        known: Vec<SenderCursor>,
    },
    SyncResponse {
        messages: Vec<MessageEnvelope>,
        has_more: bool,
    },
    Receipt {
        conversation_id: String,
        last_read_uid: [u8; 16],
    },
    GroupInvite(GroupInvite),
    GroupAccept {
        group_id: String,
    },
    Ok,
    Delivered {
        uid: [u8; 16],
    },
}

pub async fn write_frame<W>(writer: &mut W, request: &Request) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let payload = postcard::to_allocvec(request)?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(eyre!("消息帧过大：{} 字节", payload.len()));
    }
    let length = u32::try_from(payload.len()).map_err(|_| eyre!("消息帧过大"))?;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R>(reader: &mut R) -> Result<Request>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0u8; 4];
    reader.read_exact(&mut header).await?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(eyre!("消息帧过大：{length} 字节"));
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).await?;
    Ok(postcard::from_bytes(&payload)?)
}
