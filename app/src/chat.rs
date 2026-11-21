use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use eyre::Result;
use file_picker::FilePickerBackend;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{AppStore, GroupMemberData, MessageData, Viewport, spawn_task, workspace};
use runtime::{Store, StoreContact, StoreConversation, StoreConversationKind, StoreMember};

pub struct State {
    identity_id: Option<String>,
    private_conversation_id: Option<String>,
    group_conversation_id: Option<String>,
    private_generation: u64,
    group_generation: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            identity_id: None,
            private_conversation_id: None,
            group_conversation_id: None,
            private_generation: 0,
            group_generation: 0,
        }
    }
}

pub type SharedState = Arc<Mutex<State>>;

const MAX_ATTACHMENT_BYTES: usize = 100 * 1024 * 1024;

#[derive(Clone, Copy)]
enum PickKind {
    File,
    Image,
}

pub async fn load_account(
    store: &Store,
    state: &SharedState,
    identity_id: &str,
) -> Result<workspace::Workspace> {
    if let Ok(mut state) = state.lock() {
        state.identity_id = Some(identity_id.to_string());
        state.private_conversation_id = None;
        state.group_conversation_id = None;
    }
    ensure_endpoint_id(store, identity_id).await?;
    workspace::load_workspace(store, identity_id).await
}

pub async fn ensure_endpoint_id(store: &Store, identity_id: &str) -> Result<Option<Vec<u8>>> {
    let account = store
        .account(identity_id)
        .await?
        .ok_or_else(|| eyre::eyre!("账户不存在"))?;
    if let Some(endpoint_id) = account.endpoint_id {
        return Ok(Some(endpoint_id));
    }
    let key = store
        .identity_key(identity_id)
        .await?
        .ok_or_else(|| eyre::eyre!("身份密钥不存在"))?;
    let endpoint_id = runtime::identity::endpoint_id_from_secret_key(&key)?;
    store
        .set_endpoint_id(identity_id, Some(&endpoint_id))
        .await?;
    Ok(Some(endpoint_id.to_vec()))
}

pub fn apply_workspace(
    viewport: slint::Weak<Viewport>,
    identity_id: String,
    workspace: workspace::Workspace,
    online: std::collections::HashSet<[u8; 32]>,
) {
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_contacts(workspace::contacts_model(&workspace.contacts, &online));
            viewport.set_private_conversations(workspace::conversations_model(
                &workspace.private_conversations,
                &identity_id,
            ));
            viewport.set_group_conversations(workspace::conversations_model(
                &workspace.group_conversations,
                &identity_id,
            ));
            viewport.set_private_messages(empty_model::<MessageData>());
            viewport.set_group_messages(empty_model::<MessageData>());
            viewport.set_group_members(empty_model::<GroupMemberData>());
            viewport.set_private_conversation(-1);
            viewport.set_group_conversation(-1);
            viewport.set_private_draft("".into());
            viewport.set_group_draft("".into());
        }
    });
}

pub fn setup(
    viewport: &Viewport,
    app: &AppStore,
    state: &SharedState,
    online: &runtime::SharedOnline,
    network: &runtime::SharedNetwork,
) {
    viewport.on_contact_message_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |profile_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            clear_private_messages(&viewport);
            let profile_id = profile_id.to_string();
            let intent_generation = select_private(&state, None);
            spawn_task(async move {
                match app.store.ensure_private(&identity_id, &profile_id).await {
                    Ok(conversation_id) => {
                        if private_generation(&state) != intent_generation {
                            return;
                        }
                        select_private(&state, Some(conversation_id.clone()));
                        if let Err(err) = reload_private(
                            &app.store,
                            &identity_id,
                            &viewport,
                            &state,
                            Some(conversation_id.clone()),
                        )
                        .await
                        {
                            log::error!("打开私聊失败：{err:?}");
                        }
                        let _ = runtime::send_receipt(
                            &app.storage,
                            &network,
                            &identity_id,
                            &conversation_id,
                        )
                        .await;
                    }
                    Err(err) => log::error!("创建私聊失败：{err:?}"),
                }
            });
        }
    });

    viewport.on_contact_delete_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let online = online.clone();
        move |profile_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let online = online.clone();
            let profile_id = profile_id.to_string();
            spawn_task(async move {
                match app.store.remove_contact(&identity_id, &profile_id).await {
                    Ok(()) => match app.store.contacts(&identity_id).await {
                        Ok(rows) => {
                            let online = runtime::online_snapshot(&online);
                            apply_contacts(viewport, rows, &online);
                        }
                        Err(err) => log::error!("读取联系人失败：{err:?}"),
                    },
                    Err(err) => log::error!("删除联系人失败：{err:?}"),
                }
            });
        }
    });

    viewport.on_private_conversation_selected({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            clear_private_messages(&viewport);
            let conversation_id = conversation_id.to_string();
            select_private(&state, Some(conversation_id.clone()));
            spawn_task(async move {
                if let Err(err) = reload_private(
                    &app.store,
                    &identity_id,
                    &viewport,
                    &state,
                    Some(conversation_id.clone()),
                )
                .await
                {
                    log::error!("读取私聊消息失败：{err:?}");
                }
                let _ =
                    runtime::send_receipt(&app.storage, &network, &identity_id, &conversation_id)
                        .await;
            });
        }
    });

    viewport.on_group_conversation_selected({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            clear_group_messages(&viewport);
            let conversation_id = conversation_id.to_string();
            select_group(&state, Some(conversation_id.clone()));
            spawn_task(async move {
                if let Err(err) = reload_group(
                    &app.store,
                    &identity_id,
                    &viewport,
                    &state,
                    Some(conversation_id.clone()),
                )
                .await
                {
                    log::error!("读取群聊消息失败：{err:?}");
                }
                let _ = runtime::send_group_receipt(
                    &app.storage,
                    &network,
                    &identity_id,
                    &conversation_id,
                )
                .await;
            });
        }
    });

    viewport.on_private_message_send_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id, markdown| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            let conversation_id = conversation_id.to_string();
            let markdown = markdown.to_string();
            spawn_task(async move {
                if let Err(err) = runtime::send_private_markdown(
                    &app.storage,
                    &network,
                    &identity_id,
                    &conversation_id,
                    &markdown,
                    now_ms(),
                )
                .await
                {
                    log::error!("发送消息失败：{err:?}");
                    return;
                }
                clear_private_draft(&viewport, &state, &conversation_id);
                if let Err(err) = reload_private(
                    &app.store,
                    &identity_id,
                    &viewport,
                    &state,
                    Some(conversation_id),
                )
                .await
                {
                    log::error!("刷新私聊消息失败：{err:?}");
                }
            });
        }
    });

    viewport.on_group_message_send_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id, markdown| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            let conversation_id = conversation_id.to_string();
            let markdown = markdown.to_string();
            spawn_task(async move {
                if let Err(err) = runtime::send_group_markdown(
                    &app.storage,
                    &network,
                    &identity_id,
                    &conversation_id,
                    &markdown,
                    now_ms(),
                )
                .await
                {
                    log::error!("发送群消息失败：{err:?}");
                    return;
                }
                clear_group_draft(&viewport, &state, &conversation_id);
                if let Err(err) = reload_group(
                    &app.store,
                    &identity_id,
                    &viewport,
                    &state,
                    Some(conversation_id),
                )
                .await
                {
                    log::error!("刷新群聊消息失败：{err:?}");
                }
            });
        }
    });

    viewport.on_private_file_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            let conversation_id = conversation_id.to_string();
            spawn_task(async move {
                let Some((name, data)) = pick_attachment(PickKind::File).await else {
                    return;
                };
                if let Err(err) = send_attachment(
                    &app,
                    &network,
                    &identity_id,
                    &viewport,
                    &state,
                    &conversation_id,
                    false,
                    PickKind::File,
                    &name,
                    &data,
                )
                .await
                {
                    log::error!("发送文件失败：{err:?}");
                }
            });
        }
    });

    viewport.on_private_image_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            let conversation_id = conversation_id.to_string();
            spawn_task(async move {
                let Some((name, data)) = pick_attachment(PickKind::Image).await else {
                    return;
                };
                if let Err(err) = send_attachment(
                    &app,
                    &network,
                    &identity_id,
                    &viewport,
                    &state,
                    &conversation_id,
                    false,
                    PickKind::Image,
                    &name,
                    &data,
                )
                .await
                {
                    log::error!("发送图片失败：{err:?}");
                }
            });
        }
    });

    viewport.on_group_file_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            let conversation_id = conversation_id.to_string();
            spawn_task(async move {
                let Some((name, data)) = pick_attachment(PickKind::File).await else {
                    return;
                };
                if let Err(err) = send_attachment(
                    &app,
                    &network,
                    &identity_id,
                    &viewport,
                    &state,
                    &conversation_id,
                    true,
                    PickKind::File,
                    &name,
                    &data,
                )
                .await
                {
                    log::error!("发送群文件失败：{err:?}");
                }
            });
        }
    });

    viewport.on_group_image_requested({
        let viewport = viewport.as_weak();
        let app = app.clone();
        let state = state.clone();
        let network = network.clone();
        move |conversation_id| {
            let Some(identity_id) = identity(&state) else {
                return;
            };
            let viewport = viewport.clone();
            let app = app.clone();
            let state = state.clone();
            let network = network.clone();
            let conversation_id = conversation_id.to_string();
            spawn_task(async move {
                let Some((name, data)) = pick_attachment(PickKind::Image).await else {
                    return;
                };
                if let Err(err) = send_attachment(
                    &app,
                    &network,
                    &identity_id,
                    &viewport,
                    &state,
                    &conversation_id,
                    true,
                    PickKind::Image,
                    &name,
                    &data,
                )
                .await
                {
                    log::error!("发送群图片失败：{err:?}");
                }
            });
        }
    });
}

pub(crate) async fn reload_private(
    store: &Store,
    identity_id: &str,
    viewport: &slint::Weak<Viewport>,
    state: &SharedState,
    selected: Option<String>,
) -> Result<()> {
    let selected = selected.or_else(|| selected_private(state));
    let generation = private_generation(state);
    let rows = store
        .conversations(identity_id, StoreConversationKind::Private)
        .await?;
    let selected = selected.filter(|id| rows.iter().any(|row| &row.id == id));
    let loaded = match selected.as_deref() {
        Some(id) => workspace::load_messages(store, identity_id, id).await?,
        None => Vec::new(),
    };
    if private_generation(state) != generation || selected_private(state) != selected {
        return Ok(());
    }
    if let Some(latest) = loaded.last().map(|message| message.message.id) {
        if let Some(id) = selected.as_deref()
            && let Err(err) = store.mark_read(identity_id, id, Some(latest)).await
        {
            log::warn!("更新会话已读位置失败：{err:?}");
        }
    }
    let rows = store
        .conversations(identity_id, StoreConversationKind::Private)
        .await?;
    if private_generation(state) != generation || selected_private(state) != selected {
        return Ok(());
    }
    apply_private_state(
        viewport.clone(),
        identity_id.to_string(),
        rows,
        selected,
        loaded,
    );
    Ok(())
}

pub(crate) async fn reload_group(
    store: &Store,
    identity_id: &str,
    viewport: &slint::Weak<Viewport>,
    state: &SharedState,
    selected: Option<String>,
) -> Result<()> {
    let selected = selected.or_else(|| selected_group(state));
    let generation = group_generation(state);
    let rows = store
        .conversations(identity_id, StoreConversationKind::Group)
        .await?;
    let selected = selected.filter(|id| rows.iter().any(|row| &row.id == id));
    let loaded = match selected.as_deref() {
        Some(id) => workspace::load_messages(store, identity_id, id).await?,
        None => Vec::new(),
    };
    let members = match selected.as_deref() {
        Some(id) => store.members(identity_id, id).await?,
        None => Vec::new(),
    };
    if group_generation(state) != generation || selected_group(state) != selected {
        return Ok(());
    }
    if let Some(latest) = loaded.last().map(|message| message.message.id) {
        if let Some(id) = selected.as_deref()
            && let Err(err) = store.mark_read(identity_id, id, Some(latest)).await
        {
            log::warn!("更新会话已读位置失败：{err:?}");
        }
    }
    let rows = store
        .conversations(identity_id, StoreConversationKind::Group)
        .await?;
    if group_generation(state) != generation || selected_group(state) != selected {
        return Ok(());
    }
    apply_group_state(
        viewport.clone(),
        identity_id.to_string(),
        rows,
        selected,
        loaded,
        members,
    );
    Ok(())
}

fn clear_private_draft(
    viewport: &slint::Weak<Viewport>,
    state: &SharedState,
    conversation_id: &str,
) {
    if selected_private(state).as_deref() != Some(conversation_id) {
        return;
    }
    let viewport = viewport.clone();
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_private_draft("".into());
        }
    });
}

fn clear_group_draft(viewport: &slint::Weak<Viewport>, state: &SharedState, conversation_id: &str) {
    if selected_group(state).as_deref() != Some(conversation_id) {
        return;
    }
    let viewport = viewport.clone();
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_group_draft("".into());
        }
    });
}

fn clear_private_messages(viewport: &slint::Weak<Viewport>) {
    let viewport = viewport.clone();
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_private_messages(empty_model::<MessageData>());
        }
    });
}

fn clear_group_messages(viewport: &slint::Weak<Viewport>) {
    let viewport = viewport.clone();
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_group_messages(empty_model::<MessageData>());
        }
    });
}

pub(crate) fn apply_contacts(
    viewport: slint::Weak<Viewport>,
    rows: Vec<StoreContact>,
    online: &std::collections::HashSet<[u8; 32]>,
) {
    let online = online.clone();
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_contacts(workspace::contacts_model(&rows, &online));
        }
    });
}

fn apply_private_state(
    viewport: slint::Weak<Viewport>,
    identity_id: String,
    rows: Vec<StoreConversation>,
    selected: Option<String>,
    loaded: Vec<workspace::LoadedMessage>,
) {
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            let index = selected
                .as_deref()
                .and_then(|id| rows.iter().position(|row| row.id == id))
                .map(|index| index as i32)
                .unwrap_or(-1);
            let messages = workspace::messages_model(&identity_id, loaded);
            viewport.set_private_conversations(workspace::conversations_model(&rows, &identity_id));
            viewport.set_private_conversation(index);
            viewport.set_private_messages(messages);
        }
    });
}

fn apply_group_state(
    viewport: slint::Weak<Viewport>,
    identity_id: String,
    rows: Vec<StoreConversation>,
    selected: Option<String>,
    loaded: Vec<workspace::LoadedMessage>,
    members: Vec<StoreMember>,
) {
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            let index = selected
                .as_deref()
                .and_then(|id| rows.iter().position(|row| row.id == id))
                .map(|index| index as i32)
                .unwrap_or(-1);
            viewport.set_group_conversations(workspace::conversations_model(&rows, &identity_id));
            viewport.set_group_conversation(index);
            viewport.set_group_messages(workspace::messages_model(&identity_id, loaded));
            viewport.set_group_members(workspace::members_model(&members));
        }
    });
}

async fn send_attachment(
    app: &AppStore,
    network: &runtime::SharedNetwork,
    identity_id: &str,
    viewport: &slint::Weak<Viewport>,
    state: &SharedState,
    conversation_id: &str,
    group: bool,
    kind: PickKind,
    name: &str,
    data: &[u8],
) -> Result<()> {
    let image = matches!(kind, PickKind::Image);
    let blake3 = app.store.put_attachment(image, data).await?;
    let node = network.lock().await.as_ref().cloned();
    if let Some(node) = &node {
        if let Err(err) = node.add_blob(data.to_vec()).await {
            log::warn!("写入附件到网络存储失败：{err:?}");
        }
    }
    let markdown = attachment_markdown_blob(kind, name, &blake3);
    if group {
        runtime::send_group_markdown(
            &app.storage,
            network,
            identity_id,
            conversation_id,
            &markdown,
            now_ms(),
        )
        .await?;
        reload_group(
            &app.store,
            identity_id,
            viewport,
            state,
            Some(conversation_id.to_string()),
        )
        .await?;
    } else {
        runtime::send_private_markdown(
            &app.storage,
            network,
            identity_id,
            conversation_id,
            &markdown,
            now_ms(),
        )
        .await?;
        reload_private(
            &app.store,
            identity_id,
            viewport,
            state,
            Some(conversation_id.to_string()),
        )
        .await?;
    }
    Ok(())
}

async fn pick_attachment(kind: PickKind) -> Option<(String, Vec<u8>)> {
    let options = match kind {
        PickKind::File => file_picker::PickOptions {
            title: "选择文件".to_string(),
            filters: Vec::new(),
            mime_types: Vec::new(),
        },
        PickKind::Image => file_picker::PickOptions {
            title: "选择图片".to_string(),
            filters: vec![file_picker::FileFilter {
                name: "图片".to_string(),
                extensions: vec!["png".to_string(), "jpg".to_string(), "jpeg".to_string()],
            }],
            mime_types: vec!["image/png".to_string(), "image/jpeg".to_string()],
        },
    };

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let file = file_picker::desktop::PickedFile::new(options).await;
    #[cfg(target_os = "android")]
    let file = file_picker::android::PickedFile::new(options).await;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    let file = file_picker::web::PickedFile::new(options).await;

    let file = match file {
        Ok(Some(file)) => file,
        Ok(None) => return None,
        Err(err) => {
            log::warn!("选择文件失败：{err:?}");
            return None;
        }
    };
    let name = file.file_name();
    match file.read_to_end_limited(MAX_ATTACHMENT_BYTES).await {
        Ok(data) => Some((name, data)),
        Err(err) => {
            log::warn!("读取文件失败：{err:?}");
            None
        }
    }
}

fn attachment_markdown_blob(kind: PickKind, name: &str, hash: &[u8]) -> String {
    let name = escape_link_text(name);
    let hash = hex::encode(hash);
    match kind {
        PickKind::File => format!("[{name}](blob://{hash})"),
        PickKind::Image => format!("![{name}](blob://{hash})"),
    }
}

fn escape_link_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.push(' ');
        } else if "\\`*_{}[]()#+-.!|<>~&".contains(character) {
            escaped.push('\\');
            escaped.push(character);
        } else {
            escaped.push(character);
        }
    }
    escaped
}

pub(crate) fn select_private(state: &SharedState, value: Option<String>) -> u64 {
    if let Ok(mut state) = state.lock() {
        state.private_conversation_id = value;
        state.private_generation = state.private_generation.wrapping_add(1);
        return state.private_generation;
    }
    0
}

fn select_group(state: &SharedState, value: Option<String>) -> u64 {
    if let Ok(mut state) = state.lock() {
        state.group_conversation_id = value;
        state.group_generation = state.group_generation.wrapping_add(1);
        return state.group_generation;
    }
    0
}

pub(crate) fn begin_group_creation(state: &SharedState) -> u64 {
    select_group(state, None)
}

pub(crate) fn group_generation_matches(state: &SharedState, generation: u64) -> bool {
    group_generation(state) == generation
}

pub(crate) fn select_created_group(state: &SharedState, conversation_id: String) {
    select_group(state, Some(conversation_id));
}

fn private_generation(state: &SharedState) -> u64 {
    state
        .lock()
        .ok()
        .map(|state| state.private_generation)
        .unwrap_or(0)
}

fn group_generation(state: &SharedState) -> u64 {
    state
        .lock()
        .ok()
        .map(|state| state.group_generation)
        .unwrap_or(0)
}

pub(crate) fn identity(state: &SharedState) -> Option<String> {
    state
        .lock()
        .ok()
        .and_then(|state| state.identity_id.clone())
}

pub(crate) fn is_group_selected(state: &SharedState, conversation_id: &str) -> bool {
    selected_group(state).as_deref() == Some(conversation_id)
}

pub(crate) fn is_private_selected(state: &SharedState, conversation_id: &str) -> bool {
    selected_private(state).as_deref() == Some(conversation_id)
}

fn selected_private(state: &SharedState) -> Option<String> {
    state
        .lock()
        .ok()
        .and_then(|state| state.private_conversation_id.clone())
}

fn selected_group(state: &SharedState) -> Option<String> {
    state
        .lock()
        .ok()
        .and_then(|state| state.group_conversation_id.clone())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn empty_model<T: Clone + 'static>() -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::default()))
}
