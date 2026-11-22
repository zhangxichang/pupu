#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

use eyre::Result;
use futures::lock::Mutex;
use markdown::attachment_refs;
use storage::{accounts, attachments, contacts, conversations, messages};

use crate::SharedStorage;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use storage::native::Storage;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use storage::web::Storage;

#[derive(Clone)]
pub struct Store {
    storage: SharedStorage,
}

#[derive(Debug, Clone)]
pub struct StoreAccount {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub avatar: Option<Vec<u8>>,
    pub endpoint_id: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct StoreContact {
    pub profile_id: String,
    pub name: String,
    pub bio: Option<String>,
    pub avatar: Option<Vec<u8>>,
    pub endpoint_id: Option<Vec<u8>>,
    pub state: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreConversationKind {
    Private,
    Group,
}

#[derive(Debug, Clone)]
pub struct StoreConversation {
    pub id: String,
    pub kind: StoreConversationKind,
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
pub struct StoreMember {
    pub profile_id: String,
    pub name: String,
    pub avatar: Option<Vec<u8>>,
    pub last_read_message_id: Option<i64>,
    pub last_read_message_uid: Option<[u8; 16]>,
}

#[derive(Debug, Clone)]
pub struct StoreAttachment {
    pub hash: Vec<u8>,
    pub name: String,
    pub is_image: bool,
    pub size: Option<usize>,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct StoreMessage {
    pub id: i64,
    pub uid: [u8; 16],
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_avatar: Option<Vec<u8>>,
    pub markdown: String,
    pub timestamp: i64,
    pub status: String,
    pub read: bool,
    pub attachments: Vec<StoreAttachment>,
}

#[derive(Debug, Clone, Default)]
pub struct StoreWorkspace {
    pub contacts: Vec<StoreContact>,
    pub private_conversations: Vec<StoreConversation>,
    pub group_conversations: Vec<StoreConversation>,
}

impl Store {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub async fn open(data_dir: PathBuf) -> Result<Self> {
        let storage = Storage::open(data_dir).await?;
        Ok(Self {
            storage: Arc::new(Mutex::new(storage)),
        })
    }

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub async fn open_web(worker_url: impl Into<String>, data_dir: PathBuf) -> Result<Self> {
        let storage = Storage::open(worker_url, data_dir).await?;
        Ok(Self {
            storage: std::rc::Rc::new(Mutex::new(storage)),
        })
    }

    pub fn shared(&self) -> SharedStorage {
        self.storage.clone()
    }

    pub async fn accounts(&self) -> Result<Vec<StoreAccount>> {
        let mut storage = self.storage.lock().await;
        Ok(accounts::list(&mut *storage)
            .await?
            .into_iter()
            .map(|account| StoreAccount {
                id: account.id,
                name: account.name,
                bio: account.bio,
                avatar: account.avatar,
                endpoint_id: account.endpoint_id,
            })
            .collect())
    }

    pub async fn create_account(&self, name: &str, avatar: Option<&[u8]>) -> Result<StoreAccount> {
        let mut storage = self.storage.lock().await;
        let account = accounts::create(&mut *storage, name, avatar).await?;
        Ok(StoreAccount {
            id: account.id,
            name: account.name,
            bio: account.bio,
            avatar: account.avatar,
            endpoint_id: account.endpoint_id,
        })
    }

    pub async fn account(&self, id: &str) -> Result<Option<StoreAccount>> {
        let mut storage = self.storage.lock().await;
        Ok(accounts::get(&mut *storage, id)
            .await?
            .map(|account| StoreAccount {
                id: account.id,
                name: account.name,
                bio: account.bio,
                avatar: account.avatar,
                endpoint_id: account.endpoint_id,
            }))
    }

    pub async fn identity_key(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let mut storage = self.storage.lock().await;
        accounts::identity_key(&mut *storage, id).await
    }

    pub async fn set_endpoint_id(&self, id: &str, endpoint_id: Option<&[u8]>) -> Result<()> {
        let mut storage = self.storage.lock().await;
        accounts::set_endpoint_id(&mut *storage, id, endpoint_id).await
    }

    pub async fn conversations(
        &self,
        identity_id: &str,
        kind: StoreConversationKind,
    ) -> Result<Vec<StoreConversation>> {
        let mut storage = self.storage.lock().await;
        let kind = match kind {
            StoreConversationKind::Private => conversations::ConversationKind::Private,
            StoreConversationKind::Group => conversations::ConversationKind::Group,
        };
        conversation_rows(&mut *storage, identity_id, kind).await
    }

    pub async fn mark_read(
        &self,
        identity_id: &str,
        conversation_id: &str,
        message_id: Option<i64>,
    ) -> Result<()> {
        let mut storage = self.storage.lock().await;
        conversations::mark_read(&mut *storage, identity_id, conversation_id, message_id).await
    }

    pub async fn ensure_private(&self, identity_id: &str, profile_id: &str) -> Result<String> {
        let mut storage = self.storage.lock().await;
        conversations::ensure_private(&mut *storage, identity_id, profile_id).await
    }

    pub async fn remove_contact(&self, identity_id: &str, profile_id: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        contacts::remove(&mut *storage, identity_id, profile_id).await
    }

    pub async fn put_attachment(&self, image: bool, data: &[u8]) -> Result<[u8; 32]> {
        let kind = if image {
            attachments::AttachmentKind::Image
        } else {
            attachments::AttachmentKind::File
        };
        let mut storage = self.storage.lock().await;
        let (blake3, _) = attachments::put_blob(&mut *storage, kind, data).await?;
        Ok(blake3)
    }

    pub async fn contacts(&self, identity_id: &str) -> Result<Vec<StoreContact>> {
        let mut storage = self.storage.lock().await;
        Ok(contacts::list(&mut *storage, identity_id)
            .await?
            .into_iter()
            .map(|contact| StoreContact {
                profile_id: contact.profile_id,
                name: contact.name,
                bio: contact.bio,
                avatar: contact.avatar,
                endpoint_id: contact.endpoint_id,
                state: contact.state.as_str().to_string(),
            })
            .collect())
    }

    pub async fn create_contact(
        &self,
        identity_id: &str,
        name: &str,
        bio: Option<&str>,
        avatar: Option<&[u8]>,
        endpoint_id: Option<&[u8]>,
        state: &str,
    ) -> Result<StoreContact> {
        let state = contacts::ContactState::from_str(state)
            .ok_or_else(|| eyre::eyre!("联系人状态不正确"))?;
        let mut storage = self.storage.lock().await;
        let contact = contacts::create(
            &mut *storage,
            identity_id,
            contacts::NewContact {
                name,
                bio,
                avatar,
                endpoint_id,
                state,
            },
        )
        .await?;
        Ok(StoreContact {
            profile_id: contact.profile_id,
            name: contact.name,
            bio: contact.bio,
            avatar: contact.avatar,
            endpoint_id: contact.endpoint_id,
            state: contact.state.as_str().to_string(),
        })
    }

    pub async fn set_contact_state(
        &self,
        identity_id: &str,
        profile_id: &str,
        state: &str,
    ) -> Result<()> {
        let state = contacts::ContactState::from_str(state)
            .ok_or_else(|| eyre::eyre!("联系人状态不正确"))?;
        let mut storage = self.storage.lock().await;
        contacts::set_state(&mut *storage, identity_id, profile_id, state).await
    }

    pub async fn create_group(
        &self,
        identity_id: &str,
        name: &str,
        avatar: Option<&[u8]>,
        member_ids: &[String],
    ) -> Result<String> {
        let mut storage = self.storage.lock().await;
        conversations::create_group(&mut *storage, identity_id, name, avatar, member_ids).await
    }

    pub async fn set_network_id(
        &self,
        identity_id: &str,
        conversation_id: &str,
        network_id: Option<&str>,
    ) -> Result<()> {
        let mut storage = self.storage.lock().await;
        conversations::set_network_id(&mut *storage, identity_id, conversation_id, network_id).await
    }

    pub async fn workspace(&self, identity_id: &str) -> Result<StoreWorkspace> {
        let mut storage = self.storage.lock().await;
        let contacts = contacts::list(&mut *storage, identity_id)
            .await?
            .into_iter()
            .map(|contact| StoreContact {
                profile_id: contact.profile_id,
                name: contact.name,
                bio: contact.bio,
                avatar: contact.avatar,
                endpoint_id: contact.endpoint_id,
                state: contact.state.as_str().to_string(),
            })
            .collect();
        let private_conversations = conversation_rows(
            &mut *storage,
            identity_id,
            conversations::ConversationKind::Private,
        )
        .await?;
        let group_conversations = conversation_rows(
            &mut *storage,
            identity_id,
            conversations::ConversationKind::Group,
        )
        .await?;
        Ok(StoreWorkspace {
            contacts,
            private_conversations,
            group_conversations,
        })
    }

    pub async fn messages(
        &self,
        identity_id: &str,
        conversation_id: &str,
    ) -> Result<Vec<StoreMessage>> {
        let mut storage = self.storage.lock().await;
        let rows = messages::list(&mut *storage, identity_id, conversation_id).await?;
        let members = conversations::members(&mut *storage, identity_id, conversation_id).await?;
        let other_members = members
            .iter()
            .filter(|member| member.profile_id != identity_id)
            .collect::<Vec<_>>();
        let mut image_cache: HashMap<Vec<u8>, Option<attachments::Attachment>> = HashMap::new();
        let mut result = Vec::with_capacity(rows.len());
        for message in rows {
            let refs = attachment_refs(&message.markdown);
            let mut loaded = Vec::with_capacity(refs.len());
            for reference in refs {
                let (size, data) = if reference.is_image {
                    let attachment = if let Some(cached) = image_cache.get(&reference.hash) {
                        cached.clone()
                    } else {
                        let loaded = if reference.blake3 {
                            attachments::get_by_blake3(&mut *storage, &reference.hash).await?
                        } else {
                            attachments::get(&mut *storage, &reference.hash).await?
                        };
                        image_cache.insert(reference.hash.clone(), loaded.clone());
                        loaded
                    };
                    match attachment {
                        Some(attachment) => (Some(attachment.data.len()), Some(attachment.data)),
                        None => (None, None),
                    }
                } else {
                    let metadata = if reference.blake3 {
                        attachments::get_metadata_by_blake3(&mut *storage, &reference.hash).await?
                    } else {
                        attachments::get_metadata(&mut *storage, &reference.hash).await?
                    };
                    (
                        metadata.map(|meta| usize::try_from(meta.size).unwrap_or(0)),
                        None,
                    )
                };
                loaded.push(StoreAttachment {
                    hash: reference.hash,
                    name: reference.name,
                    is_image: reference.is_image,
                    size,
                    data,
                });
            }
            let read = message.sender_id == identity_id
                && !other_members.is_empty()
                && other_members
                    .iter()
                    .all(|member| member.last_read_message_id.unwrap_or(0) >= message.id);
            result.push(StoreMessage {
                id: message.id,
                uid: message.uid,
                conversation_id: message.conversation_id,
                sender_id: message.sender_id,
                sender_name: message.sender_name,
                sender_avatar: message.sender_avatar,
                markdown: message.markdown,
                timestamp: message.timestamp,
                status: message.status.as_str().to_string(),
                read,
                attachments: loaded,
            });
        }
        Ok(result)
    }

    pub async fn members(
        &self,
        identity_id: &str,
        conversation_id: &str,
    ) -> Result<Vec<StoreMember>> {
        let mut storage = self.storage.lock().await;
        Ok(
            conversations::members(&mut *storage, identity_id, conversation_id)
                .await?
                .into_iter()
                .map(|member| StoreMember {
                    profile_id: member.profile_id,
                    name: member.name,
                    avatar: member.avatar,
                    last_read_message_id: member.last_read_message_id,
                    last_read_message_uid: member.last_read_message_uid,
                })
                .collect(),
        )
    }
}

async fn conversation_rows(
    storage: &mut impl storage::StorageBackend,
    identity_id: &str,
    kind: conversations::ConversationKind,
) -> Result<Vec<StoreConversation>> {
    Ok(conversations::list(&mut *storage, identity_id, kind)
        .await?
        .into_iter()
        .map(|conversation| StoreConversation {
            id: conversation.id,
            kind: if conversation.kind == conversations::ConversationKind::Group {
                StoreConversationKind::Group
            } else {
                StoreConversationKind::Private
            },
            name: conversation.name,
            avatar: conversation.avatar,
            latest_message_id: conversation.latest_message_id,
            latest_markdown: conversation.latest_markdown,
            latest_sender_id: conversation.latest_sender_id,
            latest_sender_name: conversation.latest_sender_name,
            latest_at: conversation.latest_at,
            unread_count: conversation.unread_count,
            network_id: conversation.network_id,
            topic_id: conversation.topic_id,
        })
        .collect())
}
