use crate::protocol::{GroupInvite, MessageEnvelope, Profile, SenderCursor};

/// Events emitted by an active network node.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Connected {
        endpoint_id: [u8; 32],
    },
    Disconnected {
        endpoint_id: [u8; 32],
    },
    Hello {
        endpoint_id: [u8; 32],
        profile: Profile,
    },
    ContactRequest {
        endpoint_id: [u8; 32],
        profile: Profile,
        message: String,
    },
    ContactAccept {
        endpoint_id: [u8; 32],
        profile: Profile,
    },
    ContactReject {
        endpoint_id: [u8; 32],
        reason: String,
    },
    Message {
        endpoint_id: [u8; 32],
        envelope: MessageEnvelope,
    },
    SyncRequest {
        endpoint_id: [u8; 32],
        conversation_id: String,
        known: Vec<SenderCursor>,
    },
    SyncResponse {
        endpoint_id: [u8; 32],
        messages: Vec<MessageEnvelope>,
        has_more: bool,
    },
    Receipt {
        endpoint_id: [u8; 32],
        conversation_id: String,
        last_read_uid: [u8; 16],
    },
    GroupInvite {
        endpoint_id: [u8; 32],
        invite: GroupInvite,
    },
    Delivered {
        endpoint_id: [u8; 32],
        uid: [u8; 16],
    },
    Error {
        message: String,
    },
}
