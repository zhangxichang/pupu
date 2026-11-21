#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use std::time::Duration;
use std::{collections::HashSet, sync::Arc};

use futures::lock::Mutex;
use network::{
    EndpointAddr, EndpointId, Network, NetworkConfig, NetworkEvent, Profile, SecretKey,
    protocol::{MessageEnvelope, Request},
};
use tokio::sync::mpsc;

pub use network::{RelayUrl, identity, protocol};
pub mod api;
pub mod store;
pub use api::{Runtime, RuntimeContact, RuntimeMessage, RuntimeSnapshot};
pub use store::*;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub type SharedStorage = Arc<Mutex<storage::native::Storage>>;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub type SharedStorage = std::rc::Rc<Mutex<storage::web::Storage>>;

pub type SharedNetwork = Arc<Mutex<Option<Arc<Network>>>>;
pub type SharedOnline = Arc<std::sync::Mutex<HashSet<[u8; 32]>>>;
pub type RuntimeEvents = mpsc::Receiver<RuntimeEvent>;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    _ = tokio::spawn(future);
}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub fn spawn_task<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

/// UI-independent events emitted by the application runtime.
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    ContactsChanged,
    ConversationChanged {
        conversation_id: String,
    },
    MessageStored {
        conversation_id: String,
        group: bool,
    },
    Error(String),
}

pub fn state() -> SharedNetwork {
    Arc::new(Mutex::new(None))
}

pub fn online_state() -> SharedOnline {
    Arc::new(std::sync::Mutex::new(HashSet::new()))
}

pub fn online_snapshot(online: &SharedOnline) -> HashSet<[u8; 32]> {
    online
        .lock()
        .map(|online| online.clone())
        .unwrap_or_default()
}

/// Start the endpoint and the event/outbox workers for an account.
pub fn start(
    storage: SharedStorage,
    network: SharedNetwork,
    online: SharedOnline,
    identity_id: String,
    name: String,
    bio: Option<String>,
    relay_url: Option<RelayUrl>,
) -> RuntimeEvents {
    let (events, receiver) = mpsc::channel(256);
    spawn_task(async move {
        let key = {
            let mut storage = storage.lock().await;
            match storage::accounts::identity_key(&mut *storage, &identity_id).await {
                Ok(Some(key)) => key,
                Ok(None) => {
                    let _ = events
                        .send(RuntimeEvent::Error("账户身份密钥不存在".to_string()))
                        .await;
                    return;
                }
                Err(err) => {
                    let _ = events
                        .send(RuntimeEvent::Error(format!("读取身份密钥失败：{err:?}")))
                        .await;
                    return;
                }
            }
        };
        let key: [u8; 32] = match key.try_into() {
            Ok(key) => key,
            Err(_) => {
                let _ = events
                    .send(RuntimeEvent::Error("身份密钥长度不正确".to_string()))
                    .await;
                return;
            }
        };
        let config = NetworkConfig {
            secret_key: SecretKey::from_bytes(&key),
            profile: Profile {
                name,
                bio,
                relay_url: None,
            },
            use_relay: true,
            relay_url,
        };
        let (node, mut network_events) = match Network::start(config).await {
            Ok(value) => value,
            Err(err) => {
                let _ = events
                    .send(RuntimeEvent::Error(format!("启动网络失败：{err:?}")))
                    .await;
                return;
            }
        };
        log::info!("网络已启动：{}", node.id());
        let blobs = {
            let mut storage = storage.lock().await;
            storage::attachments::all_blobs(&mut *storage)
                .await
                .unwrap_or_default()
        };
        for (_, data) in blobs {
            if let Err(err) = node.add_blob(data).await {
                log::warn!("导入附件到网络存储失败：{err:?}");
            }
        }
        {
            let mut storage = storage.lock().await;
            if let Ok(pending) = storage::messages::pending_uids(&mut *storage).await {
                for uid in pending {
                    let queued = storage::outbox::has_message_uid(&mut *storage, &uid)
                        .await
                        .unwrap_or(false);
                    if !queued {
                        let _ = storage::messages::set_status(
                            &mut *storage,
                            &uid,
                            storage::messages::MessageStatus::Failed,
                        )
                        .await;
                    }
                }
            }
        }
        *network.lock().await = Some(Arc::new(node));

        spawn_task(outbox_loop(storage.clone(), network.clone()));
        spawn_task(presence_loop(
            storage.clone(),
            network.clone(),
            online.clone(),
            identity_id.clone(),
            events.clone(),
        ));
        while let Some(event) = network_events.recv().await {
            handle_event(event, &storage, &identity_id, &online, &network, &events).await;
        }
    });
    receiver
}

async fn handle_event(
    event: NetworkEvent,
    storage: &SharedStorage,
    identity_id: &str,
    online: &SharedOnline,
    network: &SharedNetwork,
    events: &mpsc::Sender<RuntimeEvent>,
) {
    match event {
        NetworkEvent::Connected { endpoint_id } => {
            log::info!("节点上线：{}", hex::encode(endpoint_id));
            if let Ok(mut online) = online.lock() {
                online.insert(endpoint_id);
            }
            let _ = events.send(RuntimeEvent::ContactsChanged).await;
        }
        NetworkEvent::Disconnected { endpoint_id } => {
            log::info!("节点离线：{}", hex::encode(endpoint_id));
            if let Ok(mut online) = online.lock() {
                online.remove(&endpoint_id);
            }
            let _ = events.send(RuntimeEvent::ContactsChanged).await;
        }
        NetworkEvent::Hello {
            endpoint_id,
            profile,
        } => {
            log::info!("收到握手：{} ({})", profile.name, hex::encode(endpoint_id));
            remember_peer(network, endpoint_id, profile.relay_url.as_deref()).await;
            sync_peer(storage, network, identity_id, endpoint_id).await;
        }
        NetworkEvent::ContactRequest {
            endpoint_id,
            profile,
            message: _,
        } => {
            remember_peer(network, endpoint_id, profile.relay_url.as_deref()).await;
            let rows = {
                let mut storage = storage.lock().await;
                match storage::contacts::upsert_remote(
                    &mut *storage,
                    identity_id,
                    storage::contacts::RemoteProfile {
                        name: &profile.name,
                        bio: profile.bio.as_deref(),
                        endpoint_id: &endpoint_id,
                    },
                    storage::contacts::ContactState::PendingIn,
                )
                .await
                {
                    Ok(_) => storage::contacts::list(&mut *storage, identity_id)
                        .await
                        .ok(),
                    Err(err) => {
                        log::warn!("写入联系人请求失败：{err:?}");
                        None
                    }
                }
            };
            if let Some(_rows) = rows {
                let _ = events.send(RuntimeEvent::ContactsChanged).await;
            }
        }
        NetworkEvent::ContactAccept {
            endpoint_id,
            profile,
        } => {
            remember_peer(network, endpoint_id, profile.relay_url.as_deref()).await;
            let rows = {
                let mut storage = storage.lock().await;
                let existing =
                    storage::contacts::find_by_endpoint(&mut *storage, identity_id, &endpoint_id)
                        .await
                        .ok()
                        .flatten();
                if !existing.is_some_and(|contact| {
                    matches!(
                        contact.state,
                        storage::contacts::ContactState::PendingOut
                            | storage::contacts::ContactState::PendingIn
                            | storage::contacts::ContactState::Accepted
                    )
                }) {
                    log::warn!("拒绝未预期的联系人接受消息");
                    return;
                }
                match storage::contacts::upsert_remote(
                    &mut *storage,
                    identity_id,
                    storage::contacts::RemoteProfile {
                        name: &profile.name,
                        bio: profile.bio.as_deref(),
                        endpoint_id: &endpoint_id,
                    },
                    storage::contacts::ContactState::Accepted,
                )
                .await
                {
                    Ok(_) => storage::contacts::list(&mut *storage, identity_id)
                        .await
                        .ok(),
                    Err(err) => {
                        log::warn!("更新联系人接受状态失败：{err:?}");
                        None
                    }
                }
            };
            if let Some(_rows) = rows {
                let _ = events.send(RuntimeEvent::ContactsChanged).await;
            }
        }
        NetworkEvent::ContactReject {
            endpoint_id,
            reason,
        } => {
            log::info!("联系人请求被拒绝：{}", reason);
            let rows = {
                let mut storage = storage.lock().await;
                match storage::contacts::find_by_endpoint(&mut *storage, identity_id, &endpoint_id)
                    .await
                {
                    Ok(Some(contact))
                        if contact.state == storage::contacts::ContactState::PendingOut =>
                    {
                        let _ = storage::contacts::set_state(
                            &mut *storage,
                            identity_id,
                            &contact.profile_id,
                            storage::contacts::ContactState::Rejected,
                        )
                        .await;
                        storage::contacts::list(&mut *storage, identity_id)
                            .await
                            .ok()
                    }
                    _ => None,
                }
            };
            if let Some(_rows) = rows {
                let _ = events.send(RuntimeEvent::ContactsChanged).await;
            }
        }
        NetworkEvent::Message {
            endpoint_id,
            envelope,
        } => {
            receive_message(envelope, endpoint_id, storage, identity_id, network, events).await;
        }
        NetworkEvent::SyncRequest {
            endpoint_id,
            conversation_id,
            known,
        } => {
            remember_peer(network, endpoint_id, None).await;
            handle_sync_request(
                storage,
                network,
                identity_id,
                endpoint_id,
                conversation_id,
                known,
            )
            .await;
        }
        NetworkEvent::SyncResponse {
            endpoint_id,
            messages,
            has_more,
        } => {
            handle_sync_response(
                storage,
                network,
                identity_id,
                events,
                endpoint_id,
                messages,
                has_more,
            )
            .await;
        }
        NetworkEvent::Receipt {
            endpoint_id,
            conversation_id,
            last_read_uid,
        } => {
            handle_receipt(
                storage,
                identity_id,
                events,
                endpoint_id,
                conversation_id,
                last_read_uid,
                network,
            )
            .await;
        }
        NetworkEvent::GroupInvite {
            endpoint_id,
            invite,
        } => {
            let contact = {
                let mut storage = storage.lock().await;
                storage::contacts::find_by_endpoint(&mut *storage, identity_id, &endpoint_id)
                    .await
                    .ok()
                    .flatten()
            };
            if !contact
                .is_some_and(|contact| contact.state == storage::contacts::ContactState::Accepted)
            {
                return;
            }
            let members = invite
                .members
                .iter()
                .map(|endpoint_id| storage::conversations::InviteMember {
                    endpoint_id: *endpoint_id,
                    name: "群成员".to_string(),
                })
                .collect::<Vec<_>>();
            let conversation_id = {
                let mut storage = storage.lock().await;
                storage::conversations::create_group_from_invite(
                    &mut *storage,
                    identity_id,
                    &invite.network_id,
                    &invite.name,
                    &members,
                )
                .await
                .ok()
            };
            if let Some(conversation_id) = conversation_id {
                let _ = events
                    .send(RuntimeEvent::ConversationChanged { conversation_id })
                    .await;
            }
        }
        NetworkEvent::Delivered { endpoint_id, uid } => {
            let mut storage = storage.lock().await;
            let _ = storage::outbox::remove_message_target(&mut *storage, &uid, &endpoint_id).await;
            let remaining = storage::outbox::count_for_message_uid(&mut *storage, &uid)
                .await
                .unwrap_or(0);
            if remaining == 0 {
                let _ = storage::messages::set_status(
                    &mut *storage,
                    &uid,
                    storage::messages::MessageStatus::Sent,
                )
                .await;
            }
        }
        NetworkEvent::Error { message } => {
            log::warn!("网络错误：{message}");
            let _ = events.send(RuntimeEvent::Error(message)).await;
        }
    }
}

async fn presence_loop(
    storage: SharedStorage,
    network: SharedNetwork,
    online: SharedOnline,
    identity_id: String,
    events: mpsc::Sender<RuntimeEvent>,
) {
    loop {
        refresh_presence(&storage, &network, &online, &identity_id, &events).await;
        sleep(10_000).await;
    }
}

async fn refresh_presence(
    storage: &SharedStorage,
    network: &SharedNetwork,
    online: &SharedOnline,
    identity_id: &str,
    events: &mpsc::Sender<RuntimeEvent>,
) {
    let contacts = {
        let mut storage = storage.lock().await;
        match storage::contacts::list(&mut *storage, identity_id).await {
            Ok(contacts) => contacts,
            Err(err) => {
                log::warn!("读取联系人状态失败：{err:?}");
                return;
            }
        }
    };
    let previous = online_snapshot(online);
    let mut current = HashSet::new();
    let node = network.lock().await.as_ref().cloned();
    let Some(node) = node else {
        return;
    };
    for contact in contacts {
        if contact.state != storage::contacts::ContactState::Accepted {
            continue;
        }
        let Some(endpoint_id) = contact.endpoint_id.as_deref() else {
            continue;
        };
        let Ok(endpoint_id) = <[u8; 32]>::try_from(endpoint_id) else {
            continue;
        };
        let Ok(id) = EndpointId::from_bytes(&endpoint_id) else {
            continue;
        };
        if let Ok(profile) = node.hello_to(id).await {
            remember_peer_with_node(&node, endpoint_id, profile.relay_url.as_deref());
            current.insert(endpoint_id);
        }
    }
    if current != previous {
        if let Ok(mut online) = online.lock() {
            *online = current;
        }
        let _ = events.send(RuntimeEvent::ContactsChanged).await;
    }
}

async fn remember_peer(network: &SharedNetwork, endpoint_id: [u8; 32], relay_url: Option<&str>) {
    let node = network.lock().await.as_ref().cloned();
    if let Some(node) = node {
        remember_peer_with_node(&node, endpoint_id, relay_url);
    }
}

fn remember_peer_with_node(node: &Network, endpoint_id: [u8; 32], relay_url: Option<&str>) {
    let Some(relay_url) = relay_url else {
        return;
    };
    let Ok(relay_url) = relay_url.parse::<RelayUrl>() else {
        return;
    };
    let Ok(id) = EndpointId::from_bytes(&endpoint_id) else {
        return;
    };
    node.add_peer(EndpointAddr::new(id).with_relay_url(relay_url));
}

async fn receive_message(
    envelope: MessageEnvelope,
    endpoint_id: [u8; 32],
    storage: &SharedStorage,
    identity_id: &str,
    network: &SharedNetwork,
    events: &mpsc::Sender<RuntimeEvent>,
) {
    let Some((conversation_id, _inserted)) =
        store_envelope(storage, identity_id, &envelope, Some(endpoint_id)).await
    else {
        return;
    };
    download_attachments(storage, network, &envelope.markdown, envelope.sender).await;
    let group = {
        let mut storage = storage.lock().await;
        storage::conversations::kind(&mut *storage, identity_id, &conversation_id)
            .await
            .is_ok_and(|kind| kind == storage::conversations::ConversationKind::Group)
    };
    let _ = events
        .send(RuntimeEvent::MessageStored {
            conversation_id,
            group,
        })
        .await;
}

async fn store_envelope(
    storage: &SharedStorage,
    identity_id: &str,
    envelope: &MessageEnvelope,
    transport_sender: Option<[u8; 32]>,
) -> Option<(String, bool)> {
    if transport_sender.is_some_and(|endpoint_id| endpoint_id != envelope.sender) {
        return None;
    }
    let endpoint_id = envelope.sender;
    let mut storage = storage.lock().await;
    let own_endpoint = storage::accounts::get(&mut *storage, identity_id)
        .await
        .ok()
        .flatten()
        .and_then(|account| account.endpoint_id)
        .and_then(|value| <[u8; 32]>::try_from(value).ok());
    let expected_private =
        own_endpoint.map(|own| ::network::identity::private_conversation_id(&own, &endpoint_id));
    let conversation_id = match storage::conversations::find_by_network_id(
        &mut *storage,
        identity_id,
        &envelope.conversation_id,
    )
    .await
    {
        Ok(Some(conversation_id)) => {
            if storage::conversations::kind(&mut *storage, identity_id, &conversation_id)
                .await
                .is_ok_and(|kind| kind == storage::conversations::ConversationKind::Private)
                && expected_private.as_deref() != Some(envelope.conversation_id.as_str())
            {
                log::warn!("拒绝不匹配的私聊网络 id");
                return None;
            }
            conversation_id
        }
        Ok(None) => {
            if expected_private.as_deref() != Some(envelope.conversation_id.as_str()) {
                log::warn!("拒绝未知会话的网络消息");
                return None;
            }
            let conversation_id = match storage::conversations::find_private_by_endpoint(
                &mut *storage,
                identity_id,
                &endpoint_id,
            )
            .await
            {
                Ok(Some(conversation_id)) => conversation_id,
                Ok(None) => {
                    let contact = match storage::contacts::find_by_endpoint(
                        &mut *storage,
                        identity_id,
                        &endpoint_id,
                    )
                    .await
                    {
                        Ok(Some(contact))
                            if contact.state == storage::contacts::ContactState::Accepted =>
                        {
                            contact
                        }
                        _ => return None,
                    };
                    match storage::conversations::ensure_private(
                        &mut *storage,
                        identity_id,
                        &contact.profile_id,
                    )
                    .await
                    {
                        Ok(conversation_id) => conversation_id,
                        Err(err) => {
                            log::warn!("创建网络私聊失败：{err:?}");
                            return None;
                        }
                    }
                }
                Err(err) => {
                    log::warn!("查找私聊失败：{err:?}");
                    return None;
                }
            };
            let _ = storage::conversations::set_network_id(
                &mut *storage,
                identity_id,
                &conversation_id,
                Some(&envelope.conversation_id),
            )
            .await;
            conversation_id
        }
        Err(err) => {
            log::warn!("查找网络会话失败：{err:?}");
            return None;
        }
    };
    let sender_profile_id =
        match storage::contacts::find_by_endpoint(&mut *storage, identity_id, &endpoint_id).await {
            Ok(Some(contact)) if contact.state == storage::contacts::ContactState::Accepted => {
                contact.profile_id
            }
            _ => match storage::conversations::member_profile_by_endpoint(
                &mut *storage,
                &conversation_id,
                &endpoint_id,
            )
            .await
            {
                Ok(Some(profile_id)) => profile_id,
                _ => return None,
            },
        };
    if let Ok(Some(_)) =
        storage::messages::get_by_uid(&mut *storage, identity_id, &envelope.uid).await
    {
        return Some((conversation_id, false));
    }
    let sender_seq = match i64::try_from(envelope.sender_seq) {
        Ok(sender_seq) => sender_seq,
        Err(_) => return None,
    };
    if let Err(err) = storage::messages::insert_network(
        &mut *storage,
        identity_id,
        &conversation_id,
        &sender_profile_id,
        storage::messages::NewMessage {
            uid: envelope.uid,
            sender_seq,
            status: storage::messages::MessageStatus::Sent,
            signature: Some(&envelope.signature),
            markdown: &envelope.markdown,
            timestamp: envelope.timestamp,
        },
    )
    .await
    {
        log::warn!("写入网络消息失败：{err:?}");
        return None;
    }
    Some((conversation_id, true))
}

const MAX_ATTACHMENT_BYTES: usize = 100 * 1024 * 1024;

async fn download_attachments(
    storage: &SharedStorage,
    network: &SharedNetwork,
    markdown: &str,
    provider: [u8; 32],
) {
    let references = markdown::attachment_refs(markdown);
    if references.iter().all(|reference| !reference.blake3) {
        return;
    }
    let node = network.lock().await.as_ref().cloned();
    let Some(node) = node else {
        return;
    };
    let Ok(provider) = EndpointId::from_bytes(&provider) else {
        return;
    };
    for reference in references.into_iter().filter(|reference| reference.blake3) {
        let hash: [u8; 32] = match reference.hash.as_slice().try_into() {
            Ok(hash) => hash,
            Err(_) => continue,
        };
        let exists = {
            let mut storage = storage.lock().await;
            storage::attachments::get_metadata_by_blake3(&mut *storage, &hash)
                .await
                .ok()
                .flatten()
                .is_some()
        };
        if exists {
            continue;
        }
        let download = n0_future::time::timeout(
            std::time::Duration::from_secs(60),
            node.download_blob(hash, provider),
        )
        .await;
        if !matches!(download, Ok(Ok(()))) {
            log::warn!("下载附件失败或超时");
            continue;
        }
        let read =
            n0_future::time::timeout(std::time::Duration::from_secs(60), node.read_blob(hash))
                .await;
        let Ok(Ok(data)) = read else {
            continue;
        };
        if data.len() > MAX_ATTACHMENT_BYTES {
            log::warn!("附件超过大小限制");
            continue;
        }
        let kind = if reference.is_image {
            storage::attachments::AttachmentKind::Image
        } else {
            storage::attachments::AttachmentKind::File
        };
        let mut storage = storage.lock().await;
        let _ = storage::attachments::put_blob(&mut *storage, kind, &data).await;
    }
}

async fn sync_peer(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    peer_endpoint: [u8; 32],
) {
    let requests = {
        let mut storage = storage.lock().await;
        let targets = match storage::conversations::sync_targets_for_endpoint(
            &mut *storage,
            identity_id,
            &peer_endpoint,
        )
        .await
        {
            Ok(targets) => targets,
            Err(_) => return,
        };
        let mut requests = Vec::with_capacity(targets.len());
        for target in targets {
            let senders =
                storage::conversations::sync_senders(&mut *storage, &target.conversation_id)
                    .await
                    .unwrap_or_default();
            let mut known = senders
                .into_iter()
                .map(|sender| ::network::protocol::SenderCursor {
                    endpoint_id: sender.endpoint_id,
                    last_seq: u64::try_from(sender.last_seq).unwrap_or(0),
                })
                .collect::<Vec<_>>();
            if known.is_empty() {
                let endpoints = storage::conversations::member_endpoints(
                    &mut *storage,
                    &target.conversation_id,
                )
                .await
                .unwrap_or_default();
                known = endpoints
                    .into_iter()
                    .map(|endpoint_id| ::network::protocol::SenderCursor {
                        endpoint_id,
                        last_seq: 0,
                    })
                    .collect();
            }
            if known.is_empty() {
                continue;
            }
            requests.push(Request::SyncRequest {
                conversation_id: target.network_id,
                known,
            });
        }
        requests
    };
    for sync_request in requests {
        if let Err(err) = request(network, peer_endpoint, sync_request).await {
            log::debug!("同步请求失败：{err:?}");
        }
    }
}

async fn handle_sync_request(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    peer_endpoint: [u8; 32],
    network_id: String,
    known: Vec<::network::protocol::SenderCursor>,
) {
    let outgoing = {
        let mut storage = storage.lock().await;
        let conversation_id = match storage::conversations::find_by_network_id(
            &mut *storage,
            identity_id,
            &network_id,
        )
        .await
        {
            Ok(Some(conversation_id)) => conversation_id,
            _ => return,
        };
        if !matches!(
            storage::conversations::member_profile_by_endpoint(
                &mut *storage,
                &conversation_id,
                &peer_endpoint,
            )
            .await,
            Ok(Some(_))
        ) {
            log::warn!("拒绝非会话成员的同步请求");
            return;
        }
        let own = match storage::accounts::get(&mut *storage, identity_id)
            .await
            .ok()
            .flatten()
            .and_then(|account| account.endpoint_id)
            .and_then(|value| <[u8; 32]>::try_from(value).ok())
        {
            Some(own) => own,
            None => return,
        };
        let key = match storage::accounts::identity_key(&mut *storage, identity_id).await {
            Ok(Some(key)) => key,
            _ => return,
        };
        let Ok(key) = <[u8; 32]>::try_from(key) else {
            return;
        };
        let secret = SecretKey::from_bytes(&key);
        let mut envelopes = Vec::new();
        let mut has_more = false;
        for cursor in &known {
            if envelopes.len() >= 100 {
                has_more = true;
                break;
            }
            let profile_id = match storage::conversations::member_profile_by_endpoint(
                &mut *storage,
                &conversation_id,
                &cursor.endpoint_id,
            )
            .await
            {
                Ok(Some(profile_id)) => profile_id,
                _ => continue,
            };
            let limit = i64::try_from(100 - envelopes.len()).unwrap_or(1).max(1);
            let messages = match storage::messages::after_seq(
                &mut *storage,
                &conversation_id,
                &profile_id,
                i64::try_from(cursor.last_seq).unwrap_or(0),
                limit,
            )
            .await
            {
                Ok(messages) => messages,
                Err(err) => {
                    log::warn!("读取同步消息失败：{err:?}");
                    continue;
                }
            };
            if messages.len() as i64 == limit {
                has_more = true;
            }
            for message in messages {
                let mut envelope = MessageEnvelope {
                    uid: message.uid,
                    conversation_id: network_id.clone(),
                    sender: cursor.endpoint_id,
                    sender_seq: u64::try_from(message.sender_seq).unwrap_or(0),
                    timestamp: message.timestamp,
                    markdown: message.markdown,
                    signature: message.signature.unwrap_or_default(),
                };
                if message.sender_id == identity_id {
                    envelope.sender = own;
                    envelope.signature.clear();
                    if ::network::identity::sign_envelope(&secret, &mut envelope).is_err() {
                        continue;
                    }
                } else if ::network::identity::verify_envelope(&envelope).is_err() {
                    continue;
                }
                envelopes.push(envelope);
            }
        }
        if envelopes.is_empty() {
            return;
        }
        Request::SyncResponse {
            messages: envelopes,
            has_more,
        }
    };
    if let Err(err) = request(network, peer_endpoint, outgoing).await {
        log::warn!("发送同步响应失败：{err:?}");
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_sync_response(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    events: &mpsc::Sender<RuntimeEvent>,
    peer_endpoint: [u8; 32],
    messages: Vec<MessageEnvelope>,
    has_more: bool,
) {
    let mut inserted = false;
    let mut changed = HashSet::new();
    for envelope in messages {
        if ::network::identity::verify_envelope(&envelope).is_err() {
            continue;
        }
        if let Some((conversation_id, was_inserted)) =
            store_envelope(storage, identity_id, &envelope, None).await
        {
            download_attachments(storage, network, &envelope.markdown, envelope.sender).await;
            if was_inserted {
                changed.insert(conversation_id);
            }
            inserted |= was_inserted;
        }
    }
    for conversation_id in changed {
        let _ = events
            .send(RuntimeEvent::ConversationChanged { conversation_id })
            .await;
    }
    if has_more && inserted {
        sync_peer(storage, network, identity_id, peer_endpoint).await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_receipt(
    storage: &SharedStorage,
    identity_id: &str,
    events: &mpsc::Sender<RuntimeEvent>,
    endpoint_id: [u8; 32],
    network_id: String,
    last_read_uid: [u8; 16],
    _network: &SharedNetwork,
) {
    let (changed, conversation_id) = {
        let mut storage = storage.lock().await;
        let conversation_id = match storage::conversations::find_by_network_id(
            &mut *storage,
            identity_id,
            &network_id,
        )
        .await
        {
            Ok(Some(conversation_id)) => conversation_id,
            _ => return,
        };
        let member_profile_id = match storage::conversations::member_profile_by_endpoint(
            &mut *storage,
            &conversation_id,
            &endpoint_id,
        )
        .await
        {
            Ok(Some(profile_id)) => profile_id,
            _ => return,
        };
        let Some(message_id) =
            storage::messages::get_by_uid(&mut *storage, identity_id, &last_read_uid)
                .await
                .ok()
                .flatten()
                .map(|message| message.id)
        else {
            return;
        };
        let changed = storage::conversations::mark_member_read(
            &mut *storage,
            &conversation_id,
            &member_profile_id,
            Some(message_id),
            &last_read_uid,
        )
        .await
        .is_ok();
        (changed, conversation_id)
    };
    if changed {
        let _ = events
            .send(RuntimeEvent::ConversationChanged { conversation_id })
            .await;
    }
}

async fn send_receipt_for_network(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    network_id: &str,
) -> eyre::Result<()> {
    let data = {
        let mut storage = storage.lock().await;
        let conversation_id = match storage::conversations::find_by_network_id(
            &mut *storage,
            identity_id,
            network_id,
        )
        .await?
        {
            Some(conversation_id) => conversation_id,
            None => return Ok(()),
        };
        let peer =
            storage::conversations::peer_endpoint(&mut *storage, identity_id, &conversation_id)
                .await?;
        let uid = storage::messages::latest_uid(&mut *storage, &conversation_id).await?;
        (peer, uid)
    };
    let (Some(peer), Some(uid)) = data else {
        return Ok(());
    };
    request(
        network,
        peer,
        Request::Receipt {
            conversation_id: network_id.to_string(),
            last_read_uid: uid,
        },
    )
    .await?;
    Ok(())
}

/// Send a group read receipt to every member.
pub async fn send_group_receipt(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    conversation_id: &str,
) -> eyre::Result<()> {
    let data = {
        let mut storage = storage.lock().await;
        let network_id =
            storage::conversations::network_id(&mut *storage, identity_id, conversation_id).await?;
        let uid = storage::messages::latest_uid(&mut *storage, conversation_id).await?;
        let endpoints =
            storage::conversations::member_endpoints(&mut *storage, conversation_id).await?;
        (network_id, uid, endpoints)
    };
    let (Some(network_id), Some(uid), endpoints) = data else {
        return Ok(());
    };
    let own = {
        let mut storage = storage.lock().await;
        storage::accounts::get(&mut *storage, identity_id)
            .await?
            .and_then(|account| account.endpoint_id)
            .and_then(|value| <[u8; 32]>::try_from(value).ok())
    };
    for target in endpoints {
        if Some(target) == own {
            continue;
        }
        let _ = enqueue(
            storage,
            network,
            target,
            Request::Receipt {
                conversation_id: network_id.clone(),
                last_read_uid: uid,
            },
        )
        .await;
    }
    Ok(())
}

/// Send a read receipt for the currently selected conversation.
pub async fn send_receipt(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    conversation_id: &str,
) -> eyre::Result<()> {
    let network_id = {
        let mut storage = storage.lock().await;
        storage::conversations::network_id(&mut *storage, identity_id, conversation_id).await?
    };
    let Some(network_id) = network_id else {
        return Ok(());
    };
    send_receipt_for_network(storage, network, identity_id, &network_id).await
}

/// Send a private message through the outbox, signing it first.
pub async fn send_private_markdown(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    conversation_id: &str,
    markdown: &str,
    timestamp: i64,
) -> eyre::Result<()> {
    let outgoing = {
        let mut storage = storage.lock().await;
        let peer =
            storage::conversations::peer_endpoint(&mut *storage, identity_id, conversation_id)
                .await?;
        let own = storage::accounts::get(&mut *storage, identity_id)
            .await?
            .and_then(|account| account.endpoint_id)
            .and_then(|value| <[u8; 32]>::try_from(value).ok());
        let (Some(peer), Some(own)) = (peer, own) else {
            storage::messages::send_markdown(
                &mut *storage,
                identity_id,
                conversation_id,
                markdown,
                timestamp,
            )
            .await?;
            return Ok(());
        };
        let network_id =
            storage::conversations::network_id(&mut *storage, identity_id, conversation_id).await?;
        let network_id = match network_id {
            Some(network_id) => network_id,
            None => {
                let network_id = ::network::identity::private_conversation_id(&own, &peer);
                storage::conversations::set_network_id(
                    &mut *storage,
                    identity_id,
                    conversation_id,
                    Some(&network_id),
                )
                .await?;
                network_id
            }
        };
        let message = storage::messages::prepare_outgoing(
            &mut *storage,
            identity_id,
            conversation_id,
            markdown,
            timestamp,
        )
        .await?;
        let key = storage::accounts::identity_key(&mut *storage, identity_id)
            .await?
            .ok_or_else(|| eyre::eyre!("身份密钥不存在"))?;
        let key: [u8; 32] = key
            .try_into()
            .map_err(|_| eyre::eyre!("身份密钥长度不正确"))?;
        let secret = SecretKey::from_bytes(&key);
        let mut envelope = MessageEnvelope {
            uid: message.uid,
            conversation_id: network_id,
            sender: own,
            sender_seq: u64::try_from(message.sender_seq).unwrap_or(0),
            timestamp,
            markdown: markdown.to_string(),
            signature: Vec::new(),
        };
        ::network::identity::sign_envelope(&secret, &mut envelope)?;
        storage::messages::set_signature(&mut *storage, &message.uid, &envelope.signature).await?;
        Some((peer, message.uid, envelope))
    };
    if let Some((peer, uid, envelope)) = outgoing {
        enqueue_message(storage, network, peer, uid, Request::Message(envelope)).await?;
    }
    Ok(())
}

/// Send a group message to every member with a known endpoint.
pub async fn send_group_markdown(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
    conversation_id: &str,
    markdown: &str,
    timestamp: i64,
) -> eyre::Result<()> {
    let outgoing = {
        let mut storage = storage.lock().await;
        let network_id =
            storage::conversations::network_id(&mut *storage, identity_id, conversation_id).await?;
        let Some(network_id) = network_id else {
            storage::messages::send_markdown(
                &mut *storage,
                identity_id,
                conversation_id,
                markdown,
                timestamp,
            )
            .await?;
            return Ok(());
        };
        let own = storage::accounts::get(&mut *storage, identity_id)
            .await?
            .and_then(|account| account.endpoint_id)
            .and_then(|value| <[u8; 32]>::try_from(value).ok());
        let Some(own) = own else {
            return Ok(());
        };
        let endpoints =
            storage::conversations::member_endpoints(&mut *storage, conversation_id).await?;
        let message = storage::messages::prepare_outgoing(
            &mut *storage,
            identity_id,
            conversation_id,
            markdown,
            timestamp,
        )
        .await?;
        let key = storage::accounts::identity_key(&mut *storage, identity_id)
            .await?
            .ok_or_else(|| eyre::eyre!("身份密钥不存在"))?;
        let key: [u8; 32] = key
            .try_into()
            .map_err(|_| eyre::eyre!("身份密钥长度不正确"))?;
        let secret = SecretKey::from_bytes(&key);
        let mut envelope = MessageEnvelope {
            uid: message.uid,
            conversation_id: network_id,
            sender: own,
            sender_seq: u64::try_from(message.sender_seq).unwrap_or(0),
            timestamp,
            markdown: markdown.to_string(),
            signature: Vec::new(),
        };
        ::network::identity::sign_envelope(&secret, &mut envelope)?;
        storage::messages::set_signature(&mut *storage, &message.uid, &envelope.signature).await?;
        let targets = endpoints
            .into_iter()
            .filter(|endpoint| *endpoint != own)
            .collect::<Vec<_>>();
        (targets, message.uid, envelope)
    };
    let (targets, uid, envelope) = outgoing;
    let has_targets = !targets.is_empty();
    for target in targets {
        let _ = enqueue_message(
            storage,
            network,
            target,
            uid,
            Request::Message(envelope.clone()),
        )
        .await;
    }
    if !has_targets {
        let mut storage = storage.lock().await;
        let _ = storage::messages::set_status(
            &mut *storage,
            &uid,
            storage::messages::MessageStatus::Sent,
        )
        .await;
    }
    Ok(())
}

/// Build the local profile from storage and the active network.
pub async fn local_profile(
    storage: &SharedStorage,
    network: &SharedNetwork,
    identity_id: &str,
) -> eyre::Result<Profile> {
    let mut profile = {
        let mut storage = storage.lock().await;
        let account = storage::accounts::get(&mut *storage, identity_id)
            .await?
            .ok_or_else(|| eyre::eyre!("账户不存在"))?;
        Profile {
            name: account.name,
            bio: account.bio,
            relay_url: None,
        }
    };
    if let Some(node) = network.lock().await.as_ref() {
        profile.relay_url = node.profile().relay_url.clone();
    }
    Ok(profile)
}

/// Send a request to a node, using the currently active network.
pub async fn request(
    network: &SharedNetwork,
    id: [u8; 32],
    request: Request,
) -> eyre::Result<Request> {
    let node = network
        .lock()
        .await
        .as_ref()
        .cloned()
        .ok_or_else(|| eyre::eyre!("网络尚未启动"))?;
    let id = EndpointId::from_bytes(&id).map_err(|err| eyre::eyre!("节点 ID 无效：{err}"))?;
    node.request_to(id, request).await
}

/// Queue a signed message in the outbox and try it immediately.
pub async fn enqueue_message(
    storage: &SharedStorage,
    network: &SharedNetwork,
    target_endpoint_id: [u8; 32],
    message_uid: [u8; 16],
    request: Request,
) -> eyre::Result<()> {
    let payload = postcard::to_allocvec(&request)?;
    {
        let mut storage = storage.lock().await;
        storage::outbox::enqueue(
            &mut *storage,
            storage::outbox::NewOutboxItem {
                conversation_id: None,
                target_endpoint_id: Some(&target_endpoint_id),
                message_uid: Some(&message_uid),
                kind: "message",
                payload: &payload,
                created_at: now_ms(),
                next_attempt_at: 0,
            },
        )
        .await?;
    }
    flush(storage, network).await;
    Ok(())
}

/// Queue a request in the outbox and try it immediately.
pub async fn enqueue(
    storage: &SharedStorage,
    network: &SharedNetwork,
    target_endpoint_id: [u8; 32],
    request: Request,
) -> eyre::Result<()> {
    let payload = postcard::to_allocvec(&request)?;
    {
        let mut storage = storage.lock().await;
        storage::outbox::enqueue(
            &mut *storage,
            storage::outbox::NewOutboxItem {
                conversation_id: None,
                target_endpoint_id: Some(&target_endpoint_id),
                message_uid: None,
                kind: request_kind(&request),
                payload: &payload,
                created_at: now_ms(),
                next_attempt_at: 0,
            },
        )
        .await?;
    }
    flush(storage, network).await;
    Ok(())
}

/// Send all due outbox entries once.
pub async fn flush(storage: &SharedStorage, network: &SharedNetwork) {
    let due = {
        let mut storage = storage.lock().await;
        match storage::outbox::due(&mut *storage, now_ms(), 8).await {
            Ok(items) => items,
            Err(err) => {
                log::warn!("读取 outbox 失败：{err:?}");
                return;
            }
        }
    };
    if due.is_empty() {
        return;
    }
    for item in due {
        let Some(target) = item.target_endpoint_id.as_deref() else {
            let mut storage = storage.lock().await;
            let _ = storage::outbox::remove(&mut *storage, item.id).await;
            continue;
        };
        let target: [u8; 32] = match target.try_into() {
            Ok(target) => target,
            Err(_) => {
                let mut storage = storage.lock().await;
                let _ = storage::outbox::remove(&mut *storage, item.id).await;
                continue;
            }
        };
        let outgoing: Request = match postcard::from_bytes(&item.payload) {
            Ok(request) => request,
            Err(err) => {
                log::warn!("outbox 负载损坏，删除：{err:?}");
                let mut storage = storage.lock().await;
                let _ = storage::outbox::remove(&mut *storage, item.id).await;
                continue;
            }
        };
        let result = request(network, target, outgoing).await;
        let accepted = match &result {
            Ok(Request::Error(error)) => {
                log::warn!("对方拒绝请求：{}", error.message);
                false
            }
            Ok(Request::Ack { uid }) => item.message_uid.is_none_or(|expected| expected == *uid),
            Ok(_) => true,
            Err(err) => {
                log::warn!("outbox 发送失败：{err:?}");
                false
            }
        };
        if accepted {
            let mut storage = storage.lock().await;
            let _ = storage::outbox::remove(&mut *storage, item.id).await;
            if let Some(uid) = item.message_uid {
                let remaining = storage::outbox::count_for_message_uid(&mut *storage, &uid)
                    .await
                    .unwrap_or(0);
                if remaining == 0 {
                    let _ = storage::messages::set_status(
                        &mut *storage,
                        &uid,
                        storage::messages::MessageStatus::Sent,
                    )
                    .await;
                }
            }
        } else if result.is_ok() {
            let mut storage = storage.lock().await;
            let attempts = item.attempts + 1;
            let next = now_ms() + backoff_ms(attempts);
            let _ = storage::outbox::record_attempt(&mut *storage, item.id, attempts, next).await;
        } else {
            let attempts = item.attempts + 1;
            let next = now_ms() + backoff_ms(attempts);
            let mut storage = storage.lock().await;
            let _ = storage::outbox::record_attempt(&mut *storage, item.id, attempts, next).await;
            if attempts >= 5
                && let Some(uid) = item.message_uid
            {
                let remaining = storage::outbox::count_for_message_uid(&mut *storage, &uid)
                    .await
                    .unwrap_or(1);
                if remaining <= 1 {
                    let _ = storage::messages::set_status(
                        &mut *storage,
                        &uid,
                        storage::messages::MessageStatus::Failed,
                    )
                    .await;
                }
            }
        }
    }
}

async fn outbox_loop(storage: SharedStorage, network: SharedNetwork) {
    loop {
        sleep(3_000).await;
        flush(&storage, &network).await;
    }
}

fn request_kind(request: &Request) -> &'static str {
    match request {
        Request::ContactRequest { .. } => "contact_request",
        Request::ContactAccept { .. } => "contact_accept",
        Request::ContactReject { .. } => "contact_reject",
        Request::Message(_) => "message",
        Request::Receipt { .. } => "receipt",
        Request::GroupInvite(_) => "group_invite",
        _ => "request",
    }
}

fn backoff_ms(attempts: i64) -> i64 {
    (attempts.clamp(1, 6) * 5_000).min(30_000)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
async fn sleep(milliseconds: u64) {
    tokio::time::sleep(Duration::from_millis(milliseconds)).await;
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
async fn sleep(milliseconds: u64) {
    gloo_timers::future::TimeoutFuture::new(u32::try_from(milliseconds).unwrap_or(u32::MAX)).await;
}
