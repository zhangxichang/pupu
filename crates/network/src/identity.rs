//! Account identity helpers shared by native and Web builds.

use eyre::{Result, eyre};
use iroh::SecretKey;
use sha2::{Digest, Sha256};

use crate::ALPN_PUPU;

/// Derive the iroh endpoint id (32-byte Ed25519 public key) from `identity.key`.
pub fn endpoint_id_from_secret_key(key: &[u8]) -> Result<[u8; 32]> {
    let key: [u8; 32] = key
        .try_into()
        .map_err(|_| eyre!("身份密钥长度必须为 32 字节"))?;
    let secret = SecretKey::from_bytes(&key);
    Ok(*secret.public().as_bytes())
}

/// Stable id shared by both sides of a private conversation.
///
/// The two endpoint ids are sorted first, so both sides derive the same value.
pub fn private_conversation_id(first: &[u8; 32], second: &[u8; 32]) -> String {
    let (low, high) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    let mut hasher = Sha256::new();
    hasher.update(ALPN_PUPU);
    hasher.update(low);
    hasher.update(high);
    hex::encode(hasher.finalize())
}

/// Fill `N` bytes from the platform CSPRNG.
pub fn random_bytes<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|error| eyre!("随机数生成失败：{error}"))?;
    Ok(bytes)
}

/// Sign the canonical fields of a message envelope.
pub fn sign_envelope(
    secret_key: &SecretKey,
    envelope: &mut crate::protocol::MessageEnvelope,
) -> Result<()> {
    let payload = envelope_payload(envelope);
    envelope.signature = secret_key.sign(&payload).to_bytes().to_vec();
    Ok(())
}

/// Verify that the envelope was signed by its sender.
pub fn verify_envelope(envelope: &crate::protocol::MessageEnvelope) -> Result<()> {
    if envelope.signature.len() != iroh::Signature::LENGTH {
        return Err(eyre!("消息签名长度不正确"));
    }
    let signature_bytes: [u8; iroh::Signature::LENGTH] =
        envelope
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| eyre!("消息签名长度不正确"))?;
    let signature = iroh::Signature::from_bytes(&signature_bytes);
    let sender = iroh::PublicKey::from_bytes(&envelope.sender)
        .map_err(|_| eyre!("消息发送者节点 ID 无效"))?;
    sender
        .verify(&envelope_payload(envelope), &signature)
        .map_err(|_| eyre!("消息签名验证失败"))
}

fn envelope_payload(envelope: &crate::protocol::MessageEnvelope) -> Vec<u8> {
    let mut payload =
        Vec::with_capacity(16 + envelope.conversation_id.len() + envelope.markdown.len() + 96);
    payload.extend_from_slice(b"pupu/chat/1/message");
    payload.extend_from_slice(&envelope.uid);
    payload.extend_from_slice(&(envelope.conversation_id.len() as u32).to_be_bytes());
    payload.extend_from_slice(envelope.conversation_id.as_bytes());
    payload.extend_from_slice(&envelope.sender);
    payload.extend_from_slice(&envelope.sender_seq.to_be_bytes());
    payload.extend_from_slice(&envelope.timestamp.to_be_bytes());
    payload.extend_from_slice(&(envelope.markdown.len() as u32).to_be_bytes());
    payload.extend_from_slice(envelope.markdown.as_bytes());
    payload
}
