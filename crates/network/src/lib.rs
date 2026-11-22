//! Pupu P2P network layer.
//!
//! One iroh endpoint serves the `pupu/chat/1` control protocol. Storage stays
//! the single local source of truth; this crate only moves signed envelopes.

pub mod endpoint;
pub mod events;
pub mod identity;
pub mod protocol;

pub use endpoint::{Network, NetworkConfig};
pub use events::NetworkEvent;
pub use iroh::{EndpointAddr, EndpointId, RelayUrl, SecretKey};
pub use protocol::Profile;

/// ALPN for private messages, contact requests, sync and receipts.
pub const ALPN_PUPU: &[u8] = b"pupu/chat/1";
