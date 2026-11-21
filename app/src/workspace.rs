use std::{collections::HashSet, rc::Rc};

use chrono::Datelike;
use eyre::Result;
use runtime::{
    Store, StoreContact, StoreConversation, StoreConversationKind, StoreMember, StoreMessage,
};
use slint::{ModelRc, VecModel};

use crate::{
    ContactData, ConversationItemData, GroupMemberData, MessageAttachmentData,
    MessageAttachmentKind, MessageData,
};

pub struct Workspace {
    pub contacts: Vec<StoreContact>,
    pub private_conversations: Vec<StoreConversation>,
    pub group_conversations: Vec<StoreConversation>,
}

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct LoadedAttachment {
    pub hash: Vec<u8>,
    pub name: String,
    pub is_image: bool,
    pub size: Option<usize>,
    pub decoded: Option<DecodedImage>,
}

pub struct LoadedMessage {
    pub message: StoreMessage,
    pub attachments: Vec<LoadedAttachment>,
}

pub async fn load_workspace(store: &Store, identity_id: &str) -> Result<Workspace> {
    let workspace = store.workspace(identity_id).await?;
    Ok(Workspace {
        contacts: workspace.contacts,
        private_conversations: workspace.private_conversations,
        group_conversations: workspace.group_conversations,
    })
}

pub async fn load_messages(
    store: &Store,
    identity_id: &str,
    conversation_id: &str,
) -> Result<Vec<LoadedMessage>> {
    let rows = store.messages(identity_id, conversation_id).await?;
    Ok(rows
        .into_iter()
        .map(|mut message| {
            let attachments = std::mem::take(&mut message.attachments)
                .into_iter()
                .map(|attachment| {
                    let decoded = if attachment.is_image {
                        attachment.data.as_deref().and_then(decode_image)
                    } else {
                        None
                    };
                    LoadedAttachment {
                        hash: attachment.hash,
                        name: attachment.name,
                        is_image: attachment.is_image,
                        size: attachment.size,
                        decoded,
                    }
                })
                .collect();
            LoadedMessage {
                message,
                attachments,
            }
        })
        .collect())
}

pub fn contacts_model(rows: &[StoreContact], online: &HashSet<[u8; 32]>) -> ModelRc<ContactData> {
    model(
        rows.iter()
            .map(|contact| contact_data(contact, online))
            .collect(),
    )
}

pub fn conversations_model(
    rows: &[StoreConversation],
    identity_id: &str,
) -> ModelRc<ConversationItemData> {
    model(
        rows.iter()
            .map(|conversation| conversation_data(conversation, identity_id))
            .collect(),
    )
}

pub fn messages_model(identity_id: &str, rows: Vec<LoadedMessage>) -> ModelRc<MessageData> {
    model(
        rows.into_iter()
            .map(|loaded| message_data(identity_id, loaded))
            .collect(),
    )
}

pub fn members_model(rows: &[StoreMember]) -> ModelRc<GroupMemberData> {
    model(rows.iter().map(member_data).collect())
}

fn contact_data(contact: &StoreContact, online: &HashSet<[u8; 32]>) -> ContactData {
    let avatar = contact.avatar.as_deref().and_then(crate::image_from_bytes);
    let is_online = contact
        .endpoint_id
        .as_deref()
        .and_then(|value| <[u8; 32]>::try_from(value).ok())
        .is_some_and(|endpoint_id| online.contains(&endpoint_id));
    ContactData {
        id: contact.profile_id.clone().into(),
        name: contact.name.clone().into(),
        avatar: avatar.clone().unwrap_or_default(),
        avatar_text: crate::avatar_text(&contact.name).into(),
        has_avatar: avatar.is_some(),
        signature: contact.bio.clone().unwrap_or_default().into(),
        online: is_online,
        state: contact.state.clone().into(),
    }
}

fn conversation_data(conversation: &StoreConversation, identity_id: &str) -> ConversationItemData {
    let avatar = conversation
        .avatar
        .as_deref()
        .and_then(crate::image_from_bytes);
    let latest = conversation
        .latest_markdown
        .as_deref()
        .map(crate::markdown::preview)
        .unwrap_or_default();
    ConversationItemData {
        id: conversation.id.clone().into(),
        name: conversation.name.clone().into(),
        avatar: avatar.clone().unwrap_or_default(),
        avatar_text: crate::avatar_text(&conversation.name).into(),
        has_avatar: avatar.is_some(),
        latest_message: preview_text(&latest, conversation, identity_id).into(),
        time: format_time(conversation.latest_at).into(),
        unread_count: i32::try_from(conversation.unread_count).unwrap_or(i32::MAX),
        online: false,
    }
}

fn preview_text(latest: &str, conversation: &StoreConversation, identity_id: &str) -> String {
    if latest.is_empty() {
        return String::new();
    }
    match conversation.kind {
        StoreConversationKind::Group => {
            if conversation.latest_sender_id.as_deref() == Some(identity_id) {
                format!("我：{latest}")
            } else {
                let sender = conversation.latest_sender_name.as_deref().unwrap_or("成员");
                format!("{sender}：{latest}")
            }
        }
        StoreConversationKind::Private => {
            if conversation.latest_sender_id.as_deref() == Some(identity_id) {
                format!("我：{latest}")
            } else {
                latest.to_string()
            }
        }
    }
}

fn member_data(member: &StoreMember) -> GroupMemberData {
    let avatar = member.avatar.as_deref().and_then(crate::image_from_bytes);
    GroupMemberData {
        id: member.profile_id.clone().into(),
        name: member.name.clone().into(),
        avatar: avatar.clone().unwrap_or_default(),
        avatar_text: crate::avatar_text(&member.name).into(),
        has_avatar: avatar.is_some(),
        online: false,
    }
}

fn message_data(identity_id: &str, loaded: LoadedMessage) -> MessageData {
    let analysis = crate::markdown::analyze(&loaded.message.markdown);
    let mut attachment_height = 0.0;
    let attachment_models = loaded
        .attachments
        .into_iter()
        .map(|attachment| {
            let is_image = attachment.is_image;
            let image = if is_image {
                attachment.decoded.map(image_from_decoded)
            } else {
                None
            };
            attachment_height += if is_image { 150.0 } else { 46.0 };
            MessageAttachmentData {
                kind: if is_image {
                    MessageAttachmentKind::Image
                } else {
                    MessageAttachmentKind::File
                },
                name: attachment.name.into(),
                size: attachment.size.map(format_size).unwrap_or_default().into(),
                hash: hex::encode(&attachment.hash).into(),
                image: image.clone().unwrap_or_default(),
                has_image: image.is_some(),
            }
        })
        .collect::<Vec<_>>();
    if attachment_models.len() > 1 {
        attachment_height += 2.0 * (attachment_models.len() - 1) as f32;
    }

    let avatar = loaded
        .message
        .sender_avatar
        .as_deref()
        .and_then(crate::image_from_bytes);
    let is_self = loaded.message.sender_id == identity_id;
    MessageData {
        id: i32::try_from(loaded.message.id).unwrap_or(i32::MAX),
        sender_name: loaded.message.sender_name.clone().into(),
        avatar: avatar.clone().unwrap_or_default(),
        avatar_text: crate::avatar_text(&loaded.message.sender_name).into(),
        has_avatar: avatar.is_some(),
        time: format_time(loaded.message.timestamp).into(),
        markdown: loaded.message.markdown.into(),
        body: crate::markdown::body_from_analysis(&analysis),
        has_body: analysis.has_body,
        attachments: model(attachment_models),
        attachment_height,
        is_self,
        read: loaded.message.read,
        status: loaded.message.status.into(),
    }
}

fn decode_image(bytes: &[u8]) -> Option<DecodedImage> {
    let image = image::load_from_memory(bytes).ok()?.thumbnail(512, 512);
    let image = image.to_rgba8();
    Some(DecodedImage {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

fn image_from_decoded(image: DecodedImage) -> slint::Image {
    let buffer = slint::SharedPixelBuffer::clone_from_slice(&image.rgba, image.width, image.height);
    slint::Image::from_rgba8(buffer)
}

fn model<T: Clone + 'static>(values: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(values)))
}

fn format_time(timestamp: i64) -> String {
    if timestamp <= 0 {
        return String::new();
    }
    let Some(utc) = chrono::DateTime::from_timestamp_millis(timestamp) else {
        return String::new();
    };
    let local = utc.with_timezone(&chrono::Local);
    let now = chrono::Local::now();
    let days = (now.date_naive() - local.date_naive()).num_days();
    if days == 0 {
        local.format("%H:%M").to_string()
    } else if days == 1 {
        "昨天".to_string()
    } else if (2..7).contains(&days) {
        match local.weekday() {
            chrono::Weekday::Mon => "星期一".to_string(),
            chrono::Weekday::Tue => "星期二".to_string(),
            chrono::Weekday::Wed => "星期三".to_string(),
            chrono::Weekday::Thu => "星期四".to_string(),
            chrono::Weekday::Fri => "星期五".to_string(),
            chrono::Weekday::Sat => "星期六".to_string(),
            chrono::Weekday::Sun => "星期日".to_string(),
        }
    } else {
        format!("{}月{}日", local.month(), local.day())
    }
}

fn format_size(size: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let size = size as f64;
    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{} B", size as usize)
    }
}
