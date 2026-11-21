#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use std::path::PathBuf;

use eyre::{Result, eyre};
use network::{EndpointAddr, EndpointId, RelayUrl};
use storage::accounts;
use storage::{contacts, conversations, messages};

use crate::{
    RuntimeEvents, SharedNetwork, SharedOnline, SharedStorage, Store, enqueue, local_profile,
    online_snapshot, online_state, send_private_markdown, start, state,
};

#[derive(Debug, Clone)]
pub struct RuntimeContact {
    pub endpoint_id: Option<[u8; 32]>,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeMessage {
    pub conversation_id: String,
    pub uid: [u8; 16],
    pub sender_id: String,
    pub markdown: String,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeSnapshot {
    pub contacts: Vec<RuntimeContact>,
    pub messages: Vec<RuntimeMessage>,
}

pub struct Runtime {
    identity_id: String,
    name: String,
    endpoint_id: [u8; 32],
    relay_url: Option<RelayUrl>,
    storage: SharedStorage,
    network: SharedNetwork,
    online: SharedOnline,
}

impl Runtime {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub async fn open(
        data_dir: PathBuf,
        name: String,
        relay_url: Option<RelayUrl>,
    ) -> Result<(Self, RuntimeEvents)> {
        let store = Store::open(data_dir).await?;
        Self::from_store(store, name, relay_url).await
    }

    pub async fn from_store(
        store: Store,
        name: String,
        relay_url: Option<RelayUrl>,
    ) -> Result<(Self, RuntimeEvents)> {
        let storage = store.shared();
        let identity_id = ensure_account(&storage, &name).await?;
        let key = {
            let mut storage = storage.lock().await;
            accounts::identity_key(&mut *storage, &identity_id)
                .await?
                .ok_or_else(|| eyre!("账户身份密钥不存在"))?
        };
        let key: [u8; 32] = key
            .try_into()
            .map_err(|_| eyre!("身份密钥长度必须为 32 字节"))?;
        let endpoint_id = network::identity::endpoint_id_from_secret_key(&key)?;
        {
            let mut storage = storage.lock().await;
            accounts::set_endpoint_id(&mut *storage, &identity_id, Some(&endpoint_id)).await?;
        }
        let network = state();
        let online = online_state();
        let events = start(
            storage.clone(),
            network.clone(),
            online.clone(),
            identity_id.clone(),
            name.clone(),
            None,
            relay_url.clone(),
        );
        Ok((
            Self {
                identity_id,
                name,
                endpoint_id,
                relay_url,
                storage,
                network,
                online,
            },
            events,
        ))
    }

    pub fn identity_id(&self) -> &str {
        &self.identity_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn endpoint_id(&self) -> [u8; 32] {
        self.endpoint_id
    }

    pub fn relay_url(&self) -> Option<RelayUrl> {
        self.relay_url.clone()
    }

    pub async fn ready(&self) -> bool {
        self.network.lock().await.is_some()
    }

    pub async fn snapshot(&self) -> Result<RuntimeSnapshot> {
        let mut storage = self.storage.lock().await;
        let contacts = contacts::list(&mut *storage, &self.identity_id)
            .await?
            .into_iter()
            .map(|contact| RuntimeContact {
                endpoint_id: contact
                    .endpoint_id
                    .and_then(|value| <[u8; 32]>::try_from(value).ok()),
                name: contact.name,
                state: contact.state.as_str().to_string(),
            })
            .collect();
        let mut message_views = Vec::new();
        for kind in [
            conversations::ConversationKind::Private,
            conversations::ConversationKind::Group,
        ] {
            for conversation in conversations::list(&mut *storage, &self.identity_id, kind).await? {
                for message in
                    messages::list(&mut *storage, &self.identity_id, &conversation.id).await?
                {
                    message_views.push(RuntimeMessage {
                        conversation_id: message.conversation_id,
                        uid: message.uid,
                        sender_id: message.sender_id,
                        markdown: message.markdown,
                        status: message.status.as_str().to_string(),
                    });
                }
            }
        }
        Ok(RuntimeSnapshot {
            contacts,
            messages: message_views,
        })
    }

    pub async fn contact_request(&self, target: [u8; 32], message: String) -> Result<()> {
        {
            let mut storage = self.storage.lock().await;
            if contacts::find_by_endpoint(&mut *storage, &self.identity_id, &target)
                .await?
                .is_none()
            {
                contacts::create(
                    &mut *storage,
                    &self.identity_id,
                    contacts::NewContact {
                        name: &hex::encode(target),
                        bio: None,
                        avatar: None,
                        endpoint_id: Some(&target),
                        state: contacts::ContactState::PendingOut,
                    },
                )
                .await?;
            }
        }
        self.remember_peer(target).await?;
        let profile = local_profile(&self.storage, &self.network, &self.identity_id).await?;
        enqueue(
            &self.storage,
            &self.network,
            target,
            network::protocol::Request::ContactRequest { profile, message },
        )
        .await?;
        Ok(())
    }

    pub async fn accept_contact(&self, target: [u8; 32]) -> Result<()> {
        let contact = {
            let mut storage = self.storage.lock().await;
            contacts::find_by_endpoint(&mut *storage, &self.identity_id, &target)
                .await?
                .ok_or_else(|| eyre!("联系人不存在"))?
        };
        self.remember_peer(target).await?;
        let profile = local_profile(&self.storage, &self.network, &self.identity_id).await?;
        enqueue(
            &self.storage,
            &self.network,
            target,
            network::protocol::Request::ContactAccept { profile },
        )
        .await?;
        {
            let mut storage = self.storage.lock().await;
            contacts::set_state(
                &mut *storage,
                &self.identity_id,
                &contact.profile_id,
                contacts::ContactState::Accepted,
            )
            .await?;
        }
        self.ensure_private(target, &contact.profile_id).await?;
        Ok(())
    }

    pub async fn send_message(&self, target: [u8; 32], markdown: String) -> Result<()> {
        let contact = {
            let mut storage = self.storage.lock().await;
            contacts::find_by_endpoint(&mut *storage, &self.identity_id, &target)
                .await?
                .ok_or_else(|| eyre!("联系人不存在"))?
        };
        if contact.state != contacts::ContactState::Accepted {
            return Err(eyre!("联系人尚未接受请求"));
        }
        let conversation_id = self.ensure_private(target, &contact.profile_id).await?;
        send_private_markdown(
            &self.storage,
            &self.network,
            &self.identity_id,
            &conversation_id,
            &markdown,
            chrono::Utc::now().timestamp_millis(),
        )
        .await?;
        Ok(())
    }

    pub async fn online(&self) -> Vec<[u8; 32]> {
        online_snapshot(&self.online).into_iter().collect()
    }

    async fn ensure_private(&self, target: [u8; 32], profile_id: &str) -> Result<String> {
        let mut storage = self.storage.lock().await;
        let conversation_id =
            conversations::ensure_private(&mut *storage, &self.identity_id, profile_id).await?;
        let network_id = network::identity::private_conversation_id(&self.endpoint_id, &target);
        conversations::set_network_id(
            &mut *storage,
            &self.identity_id,
            &conversation_id,
            Some(&network_id),
        )
        .await?;
        Ok(conversation_id)
    }

    async fn remember_peer(&self, target: [u8; 32]) -> Result<()> {
        let Some(node) = self.network.lock().await.as_ref().cloned() else {
            return Err(eyre!("网络未启动"));
        };
        let relay_url = self.relay_url.clone().or_else(|| {
            node.profile()
                .relay_url
                .as_deref()
                .and_then(|value| value.parse().ok())
        });
        if let Some(relay_url) = relay_url {
            let endpoint_id =
                EndpointId::from_bytes(&target).map_err(|_| eyre!("节点 ID 格式错误"))?;
            node.add_peer(EndpointAddr::new(endpoint_id).with_relay_url(relay_url));
        }
        Ok(())
    }
}

async fn ensure_account(storage: &SharedStorage, name: &str) -> Result<String> {
    let mut storage = storage.lock().await;
    if let Some(account) = accounts::list(&mut *storage).await?.into_iter().next() {
        return Ok(account.id);
    }
    Ok(accounts::create(&mut *storage, name, None).await?.id)
}
