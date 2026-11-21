#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    all(target_family = "wasm", target_os = "unknown"),
)))]
compile_error!("Unsupported target platform");

mod chat;
mod contributors;
mod markdown;
mod runtime_bridge;
mod virtual_list_layout;
mod window_resize;
mod workspace;

use std::{
    borrow::Cow,
    future::Future,
    sync::{Arc, Mutex},
};

use eyre::Result;
use file_picker::FilePickerBackend;
use slint::{Model, fontique_012};

use paths::AppPaths;

slint::include_modules!();

const MAX_AVATAR_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct AppStore {
    pub store: runtime::Store,
    pub storage: runtime::SharedStorage,
}

impl AppStore {
    fn new(store: runtime::Store) -> Self {
        let storage = store.shared();
        Self { store, storage }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
pub(crate) fn spawn_task<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    _ = tokio::spawn(future);
}
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub(crate) fn spawn_task<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

impl From<PickOptions> for file_picker::PickOptions {
    fn from(options: PickOptions) -> Self {
        file_picker::PickOptions {
            title: options.title.to_string(),
            filters: options
                .filters
                .iter()
                .map(|filter| file_picker::FileFilter {
                    name: filter.name.to_string(),
                    extensions: filter
                        .extensions
                        .iter()
                        .map(|ext| ext.to_string())
                        .collect(),
                })
                .collect(),
            mime_types: options
                .mime_types
                .iter()
                .map(|mime| mime.to_string())
                .collect(),
        }
    }
}

pub struct AppConfig {
    pub paths: AppPaths,
    pub font_data: Cow<'static, [u8]>,
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub storage_worker_url: String,
}

pub async fn main(config: AppConfig) -> Result<()> {
    setup_backend()?;
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let store = runtime::Store::open(config.paths.data_dir).await?;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    let store = runtime::Store::open_web(config.storage_worker_url, config.paths.data_dir).await?;
    setup_font(config.font_data)?;
    let viewport = setup_viewport(AppStore::new(store)).await?;
    viewport.run()?;
    Ok(())
}

fn setup_backend() -> Result<()> {
    let backend = slint::BackendSelector::new();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let backend = backend
        .backend_name("winit".to_string())
        .renderer_name("skia".to_string())
        .with_winit_window_attributes_hook(|attributes| attributes.with_transparent(true));
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let backend = {
        use slint::wgpu_30::{WGPUConfiguration, WGPUSettings, wgpu};

        let mut settings = WGPUSettings::default();
        #[cfg(target_os = "windows")]
        {
            settings.backends = wgpu::Backends::DX12;
            settings.backend_options.dx12.presentation_system =
                wgpu::Dx12SwapchainKind::DxgiFromVisual;
        }
        #[cfg(target_os = "linux")]
        {
            settings.backends = wgpu::Backends::VULKAN;
        }
        #[cfg(target_os = "android")]
        {
            settings.backends = wgpu::Backends::VULKAN;
            settings.power_preference = wgpu::PowerPreference::LowPower;
        }
        backend.require_wgpu_30(WGPUConfiguration::Automatic(settings))
    };
    backend.select()?;
    Ok(())
}

fn setup_font(font_data: Cow<'static, [u8]>) -> Result<()> {
    let blob = match font_data {
        Cow::Borrowed(data) => fontique_012::fontique::Blob::new(Arc::new(data)),
        Cow::Owned(data) => fontique_012::fontique::Blob::new(Arc::new(data)),
    };
    if fontique_012::shared_collection()
        .register_fonts(blob, None)
        .is_empty()
    {
        return Err(eyre::eyre!("字体注册失败"));
    }
    Ok(())
}

async fn setup_viewport(store: AppStore) -> Result<Viewport> {
    let viewport = Viewport::new()?;
    let chat_state = Arc::new(Mutex::new(chat::State::default()));
    let network_state = runtime::state();
    let online_state = runtime::online_state();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    viewport.set_platform(AppPlatform::Desktop);
    #[cfg(target_os = "android")]
    viewport.set_platform(AppPlatform::Mobile);
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    viewport.set_platform(AppPlatform::Web);
    #[cfg(any(target_os = "android", target_os = "windows", target_os = "linux"))]
    viewport.set_app_version(env!("APP_VERSION").into());
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use slint::{platform::WindowEvent, winit_030::WinitWindowAccessor};

        viewport.on_drag_window({
            let viewport = viewport.as_weak();
            move || {
                if let Some(viewport) = viewport.upgrade() {
                    _ = viewport
                        .window()
                        .with_winit_window(|window| window.drag_window());
                }
                _ = viewport.upgrade_in_event_loop(|viewport| {
                    _ = viewport
                        .window()
                        .dispatch_event_with_result(WindowEvent::PointerExited);
                });
            }
        });
        viewport.on_resize_window({
            let viewport = viewport.as_weak();
            move |direction| {
                if let Some(viewport) = viewport.upgrade() {
                    _ = viewport
                        .window()
                        .with_winit_window(|window| window.drag_resize_window(direction.into()));
                }
                _ = viewport.upgrade_in_event_loop(|viewport| {
                    _ = viewport
                        .window()
                        .dispatch_event_with_result(WindowEvent::PointerExited);
                });
            }
        });
    }
    #[cfg(any(target_os = "android", target_os = "windows", target_os = "linux"))]
    viewport.on_get_latest_version(|| "0.2.0".into());
    #[cfg(any(target_os = "android", target_os = "windows", target_os = "linux"))]
    viewport.on_compare_versions(|a, b| {
        let ordering = match (
            semver::Version::parse(a.as_str()),
            semver::Version::parse(b.as_str()),
        ) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            _ => std::cmp::Ordering::Equal,
        };
        match ordering {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    });
    viewport.on_open_url(|url| {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            log::warn!("拒绝打开非 HTTP(S) 链接：{url}");
            return;
        }
        if let Err(err) = webbrowser::open(url.as_str()) {
            log::warn!("打开链接失败：{err}");
        }
    });
    viewport.on_copy_node_id({
        let viewport = viewport.as_weak();
        move || {
            if let Some(viewport) = viewport.upgrade() {
                copy_text(viewport.get_node_id().as_str());
            }
        }
    });
    let pending_avatar = Arc::new(std::sync::Mutex::new(None::<Vec<u8>>));
    viewport.on_pick_register_avatar({
        let viewport = viewport.as_weak();
        let pending_avatar = pending_avatar.clone();
        move |options| {
            let viewport = viewport.clone();
            let pending_avatar = pending_avatar.clone();
            let options = options.into();
            spawn_task(async move {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                let file = file_picker::desktop::PickedFile::new(options).await;
                #[cfg(target_os = "android")]
                let file = file_picker::android::PickedFile::new(options).await;
                #[cfg(all(target_family = "wasm", target_os = "unknown"))]
                let file = file_picker::web::PickedFile::new(options).await;
                let file = match file {
                    Ok(Some(file)) => file,
                    Ok(None) => return,
                    Err(err) => {
                        log::warn!("选择文件失败：{err:?}");
                        return;
                    }
                };
                let bytes = match file.read_to_end_limited(MAX_AVATAR_BYTES).await {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        log::warn!("读取文件失败：{err:?}");
                        return;
                    }
                };
                let Ok(image) = image::load_from_memory(&bytes) else {
                    log::warn!("头像图片解码失败");
                    return;
                };
                let image = image.thumbnail(256, 256).to_rgba8();
                let buffer = slint::SharedPixelBuffer::clone_from_slice(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                );
                if let Ok(mut pending) = pending_avatar.lock() {
                    *pending = Some(bytes);
                }
                _ = slint::invoke_from_event_loop(move || {
                    if let Some(viewport) = viewport.upgrade() {
                        viewport.set_register_avatar_image(slint::Image::from_rgba8(buffer));
                        viewport.set_register_has_avatar(true);
                    }
                });
            });
        }
    });
    viewport.on_register_requested({
        let viewport = viewport.as_weak();
        let store = store.clone();
        let pending_avatar = pending_avatar.clone();
        move |username| {
            let viewport = viewport.clone();
            let store = store.clone();
            let pending_avatar = pending_avatar.clone();
            let username = username.to_string();
            spawn_task(async move {
                let avatar = pending_avatar
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.take());
                match store
                    .store
                    .create_account(&username, avatar.as_deref())
                    .await
                {
                    Ok(_) => match store.store.accounts().await {
                        Ok(accounts) => {
                            set_accounts(viewport, accounts);
                        }
                        Err(err) => log::error!("读取账户失败：{err:?}"),
                    },
                    Err(err) => log::error!("注册失败：{err:?}"),
                }
            });
        }
    });
    let pending_contact_avatar = Arc::new(Mutex::new(None::<Vec<u8>>));
    viewport.on_pick_contact_avatar({
        let viewport = viewport.as_weak();
        let pending_contact_avatar = pending_contact_avatar.clone();
        move |options| {
            let viewport = viewport.clone();
            let pending_contact_avatar = pending_contact_avatar.clone();
            let options = options.into();
            spawn_task(async move {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                let file = file_picker::desktop::PickedFile::new(options).await;
                #[cfg(target_os = "android")]
                let file = file_picker::android::PickedFile::new(options).await;
                #[cfg(all(target_family = "wasm", target_os = "unknown"))]
                let file = file_picker::web::PickedFile::new(options).await;
                let file = match file {
                    Ok(Some(file)) => file,
                    Ok(None) => return,
                    Err(err) => {
                        log::warn!("选择联系人头像失败：{err:?}");
                        return;
                    }
                };
                let bytes = match file.read_to_end_limited(MAX_AVATAR_BYTES).await {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        log::warn!("读取联系人头像失败：{err:?}");
                        return;
                    }
                };
                let Ok(image) = image::load_from_memory(&bytes) else {
                    log::warn!("联系人头像解码失败");
                    return;
                };
                let image = image.thumbnail(256, 256).to_rgba8();
                let buffer = slint::SharedPixelBuffer::clone_from_slice(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                );
                if let Ok(mut pending) = pending_contact_avatar.lock() {
                    *pending = Some(bytes);
                }
                _ = slint::invoke_from_event_loop(move || {
                    if let Some(viewport) = viewport.upgrade() {
                        viewport.set_contact_avatar_image(slint::Image::from_rgba8(buffer));
                        viewport.set_contact_has_avatar(true);
                    }
                });
            });
        }
    });
    viewport.on_contact_add_requested({
        let viewport = viewport.as_weak();
        let store = store.clone();
        let chat_state = chat_state.clone();
        let network_state = network_state.clone();
        let online_state = online_state.clone();
        let pending_contact_avatar = pending_contact_avatar.clone();
        move |name, bio, endpoint_id| {
            let Some(identity_id) = chat::identity(&chat_state) else {
                return;
            };
            let endpoint_id = match parse_endpoint_id(endpoint_id.as_str()) {
                Ok(endpoint_id) => endpoint_id,
                Err(err) => {
                    log::warn!("节点 ID 无效：{err:?}");
                    return;
                }
            };
            let viewport = viewport.clone();
            let store = store.clone();
            let network_state = network_state.clone();
            let online_state = online_state.clone();
            let pending_contact_avatar = pending_contact_avatar.clone();
            let name = name.to_string();
            let bio = bio.to_string();
            spawn_task(async move {
                let avatar = pending_contact_avatar
                    .lock()
                    .ok()
                    .and_then(|pending| pending.clone());
                let bio = (!bio.trim().is_empty()).then_some(bio.as_str());
                let created = store
                    .store
                    .create_contact(
                        &identity_id,
                        &name,
                        bio,
                        avatar.as_deref(),
                        Some(&endpoint_id),
                        "pending_out",
                    )
                    .await;
                let rows = store.store.contacts(&identity_id).await.ok();
                if let Some(rows) = rows {
                    if let Ok(mut pending) = pending_contact_avatar.lock() {
                        *pending = None;
                    }
                    let online = runtime::online_snapshot(&online_state);
                    chat::apply_contacts(viewport.clone(), rows, &online);
                    _ = slint::invoke_from_event_loop(move || {
                        if let Some(viewport) = viewport.upgrade() {
                            viewport.set_contact_avatar_image(Default::default());
                            viewport.set_contact_has_avatar(false);
                        }
                    });
                }
                match created {
                    Ok(_) => {
                        let profile = match runtime::local_profile(
                            &store.storage,
                            &network_state,
                            &identity_id,
                        )
                        .await
                        {
                            Ok(profile) => profile,
                            Err(err) => {
                                log::warn!("读取本机资料失败：{err:?}");
                                return;
                            }
                        };
                        if let Err(err) = runtime::enqueue(
                            &store.storage,
                            &network_state,
                            endpoint_id,
                            runtime::protocol::Request::ContactRequest {
                                profile,
                                message: String::new(),
                            },
                        )
                        .await
                        {
                            log::warn!("发送联系人请求失败：{err:?}");
                        }
                    }
                    Err(err) => log::error!("添加联系人失败：{err:?}"),
                }
            });
        }
    });
    viewport.on_contact_accept_requested({
        let viewport = viewport.as_weak();
        let store = store.clone();
        let chat_state = chat_state.clone();
        let network_state = network_state.clone();
        let online_state = online_state.clone();
        move |profile_id| {
            let Some(identity_id) = chat::identity(&chat_state) else {
                return;
            };
            let viewport = viewport.clone();
            let store = store.clone();
            let network_state = network_state.clone();
            let online_state = online_state.clone();
            let profile_id = profile_id.to_string();
            spawn_task(async move {
                let _ = store
                    .store
                    .set_contact_state(&identity_id, &profile_id, "accepted")
                    .await;
                let rows = store.store.contacts(&identity_id).await.ok();
                let endpoint_id = rows.as_ref().and_then(|rows| {
                    rows.iter()
                        .find(|contact| contact.profile_id == profile_id)
                        .and_then(|contact| contact.endpoint_id.clone())
                });
                if let Some(rows) = rows {
                    let online = runtime::online_snapshot(&online_state);
                    chat::apply_contacts(viewport, rows, &online);
                }
                let Some(endpoint_id) = endpoint_id.and_then(|value| value.try_into().ok()) else {
                    return;
                };
                let profile = match runtime::local_profile(
                    &store.storage,
                    &network_state,
                    &identity_id,
                )
                .await
                {
                    Ok(profile) => profile,
                    Err(err) => {
                        log::warn!("读取本机资料失败：{err:?}");
                        return;
                    }
                };
                let _ = runtime::enqueue(
                    &store.storage,
                    &network_state,
                    endpoint_id,
                    runtime::protocol::Request::ContactAccept { profile },
                )
                .await;
            });
        }
    });
    viewport.on_contact_reject_requested({
        let viewport = viewport.as_weak();
        let store = store.clone();
        let chat_state = chat_state.clone();
        let network_state = network_state.clone();
        let online_state = online_state.clone();
        move |profile_id| {
            let Some(identity_id) = chat::identity(&chat_state) else {
                return;
            };
            let viewport = viewport.clone();
            let store = store.clone();
            let network_state = network_state.clone();
            let online_state = online_state.clone();
            let profile_id = profile_id.to_string();
            spawn_task(async move {
                let _ = store
                    .store
                    .set_contact_state(&identity_id, &profile_id, "rejected")
                    .await;
                let rows = store.store.contacts(&identity_id).await.ok();
                let endpoint_id = rows.as_ref().and_then(|rows| {
                    rows.iter()
                        .find(|contact| contact.profile_id == profile_id)
                        .and_then(|contact| contact.endpoint_id.clone())
                });
                if let Some(rows) = rows {
                    let online = runtime::online_snapshot(&online_state);
                    chat::apply_contacts(viewport, rows, &online);
                }
                let Some(endpoint_id) = endpoint_id.and_then(|value| value.try_into().ok()) else {
                    return;
                };
                let _ = runtime::enqueue(
                    &store.storage,
                    &network_state,
                    endpoint_id,
                    runtime::protocol::Request::ContactReject {
                        reason: "已拒绝".to_string(),
                    },
                )
                .await;
            });
        }
    });
    viewport.on_contact_add_cancelled({
        let viewport = viewport.as_weak();
        let pending_contact_avatar = pending_contact_avatar.clone();
        move || {
            if let Ok(mut pending) = pending_contact_avatar.lock() {
                *pending = None;
            }
            let viewport = viewport.clone();
            _ = slint::invoke_from_event_loop(move || {
                if let Some(viewport) = viewport.upgrade() {
                    viewport.set_contact_avatar_image(Default::default());
                    viewport.set_contact_has_avatar(false);
                }
            });
        }
    });
    let pending_group_avatar = Arc::new(Mutex::new(None::<Vec<u8>>));
    viewport.on_pick_group_avatar({
        let viewport = viewport.as_weak();
        let pending_group_avatar = pending_group_avatar.clone();
        move |options| {
            let viewport = viewport.clone();
            let pending_group_avatar = pending_group_avatar.clone();
            let options = options.into();
            spawn_task(async move {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                let file = file_picker::desktop::PickedFile::new(options).await;
                #[cfg(target_os = "android")]
                let file = file_picker::android::PickedFile::new(options).await;
                #[cfg(all(target_family = "wasm", target_os = "unknown"))]
                let file = file_picker::web::PickedFile::new(options).await;
                let file = match file {
                    Ok(Some(file)) => file,
                    Ok(None) => return,
                    Err(err) => {
                        log::warn!("选择群头像失败：{err:?}");
                        return;
                    }
                };
                let bytes = match file.read_to_end_limited(MAX_AVATAR_BYTES).await {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        log::warn!("读取群头像失败：{err:?}");
                        return;
                    }
                };
                let Ok(image) = image::load_from_memory(&bytes) else {
                    log::warn!("群头像解码失败");
                    return;
                };
                let image = image.thumbnail(256, 256).to_rgba8();
                let buffer = slint::SharedPixelBuffer::clone_from_slice(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                );
                if let Ok(mut pending) = pending_group_avatar.lock() {
                    *pending = Some(bytes);
                }
                _ = slint::invoke_from_event_loop(move || {
                    if let Some(viewport) = viewport.upgrade() {
                        viewport.set_group_avatar_image(slint::Image::from_rgba8(buffer));
                        viewport.set_group_has_avatar(true);
                    }
                });
            });
        }
    });
    viewport.on_group_create_requested({
        let viewport = viewport.as_weak();
        let store = store.clone();
        let chat_state = chat_state.clone();
        let network_state = network_state.clone();
        let pending_group_avatar = pending_group_avatar.clone();
        move |name| {
            let Some(identity_id) = chat::identity(&chat_state) else {
                return;
            };
            let viewport = viewport.clone();
            let store = store.clone();
            let chat_state = chat_state.clone();
            let network_state = network_state.clone();
            let pending_group_avatar = pending_group_avatar.clone();
            let intent_generation = chat::begin_group_creation(&chat_state);
            let name = name.to_string();
            spawn_task(async move {
                let avatar = pending_group_avatar
                    .lock()
                    .ok()
                    .and_then(|pending| pending.clone());
                let contacts = match store.store.contacts(&identity_id).await {
                    Ok(contacts) => contacts,
                    Err(err) => {
                        log::error!("读取联系人失败：{err:?}");
                        return;
                    }
                };
                let member_ids = contacts
                    .iter()
                    .map(|contact| contact.profile_id.clone())
                    .collect::<Vec<_>>();
                match store
                    .store
                    .create_group(&identity_id, &name, avatar.as_deref(), &member_ids)
                    .await
                {
                    Ok(conversation_id) => {
                        if !chat::group_generation_matches(&chat_state, intent_generation) {
                            return;
                        }
                        if let Ok(mut pending) = pending_group_avatar.lock() {
                            *pending = None;
                        }
                        let network_id = match runtime::identity::random_bytes::<16>() {
                            Ok(bytes) => hex::encode(bytes),
                            Err(err) => {
                                log::error!("生成群网络 ID 失败：{err:?}");
                                return;
                            }
                        };
                        if let Err(err) = store
                            .store
                            .set_network_id(&identity_id, &conversation_id, Some(&network_id))
                            .await
                        {
                            log::error!("设置群网络 id 失败：{err:?}");
                            return;
                        }
                        let own_endpoint = store
                            .store
                            .account(&identity_id)
                            .await
                            .ok()
                            .flatten()
                            .and_then(|account| account.endpoint_id)
                            .and_then(|value| <[u8; 32]>::try_from(value).ok());
                        let mut invite_members = own_endpoint.into_iter().collect::<Vec<_>>();
                        let mut targets = Vec::new();
                        for contact in &contacts {
                            if let Some(endpoint) = contact
                                .endpoint_id
                                .as_deref()
                                .and_then(|value| <[u8; 32]>::try_from(value).ok())
                            {
                                invite_members.push(endpoint);
                                if Some(endpoint) != own_endpoint {
                                    targets.push(endpoint);
                                }
                            }
                        }
                        let invite = runtime::protocol::GroupInvite {
                            group_id: network_id.clone(),
                            network_id: network_id.clone(),
                            name: name.clone(),
                            members: invite_members,
                            avatar_blake3: None,
                        };
                        chat::select_created_group(&chat_state, conversation_id.clone());
                        if let Err(err) = chat::reload_group(
                            &store.store,
                            &identity_id,
                            &viewport,
                            &chat_state,
                            Some(conversation_id),
                        )
                        .await
                        {
                            log::error!("刷新群聊失败：{err:?}");
                        }
                        for target in targets {
                            let _ = runtime::enqueue(
                                &store.storage,
                                &network_state,
                                target,
                                runtime::protocol::Request::GroupInvite(invite.clone()),
                            )
                            .await;
                        }
                        _ = slint::invoke_from_event_loop(move || {
                            if let Some(viewport) = viewport.upgrade() {
                                viewport.set_group_avatar_image(Default::default());
                                viewport.set_group_has_avatar(false);
                            }
                        });
                    }
                    Err(err) => log::error!("创建群聊失败：{err:?}"),
                }
            });
        }
    });
    viewport.on_group_create_cancelled({
        let viewport = viewport.as_weak();
        let pending_group_avatar = pending_group_avatar.clone();
        move || {
            if let Ok(mut pending) = pending_group_avatar.lock() {
                *pending = None;
            }
            let viewport = viewport.clone();
            _ = slint::invoke_from_event_loop(move || {
                if let Some(viewport) = viewport.upgrade() {
                    viewport.set_group_avatar_image(Default::default());
                    viewport.set_group_has_avatar(false);
                }
            });
        }
    });
    viewport.on_login_requested({
        let viewport = viewport.as_weak();
        let store = store.clone();
        let chat_state = chat_state.clone();
        let network_state = network_state.clone();
        let online_state = online_state.clone();
        move |account_id| {
            let viewport = viewport.clone();
            let store = store.clone();
            let chat_state = chat_state.clone();
            let network_state = network_state.clone();
            let online_state = online_state.clone();
            let account_id = account_id.to_string();
            spawn_task(async move {
                match store.store.account(&account_id).await {
                    Ok(Some(account)) => {
                        let identity_id = account.id.clone();
                        let workspace =
                            match chat::load_account(&store.store, &chat_state, &identity_id).await
                            {
                                Ok(workspace) => workspace,
                                Err(err) => {
                                    log::error!("读取工作区失败：{err:?}");
                                    workspace::Workspace {
                                        contacts: Vec::new(),
                                        private_conversations: Vec::new(),
                                        group_conversations: Vec::new(),
                                    }
                                }
                            };
                        let account = match store.store.account(&identity_id).await {
                            Ok(Some(account)) => account,
                            Ok(None) => account,
                            Err(err) => {
                                log::error!("读取账户节点信息失败：{err:?}");
                                account
                            }
                        };
                        set_current_account(viewport.clone(), account.clone());
                        let online = runtime::online_snapshot(&online_state);
                        chat::apply_workspace(
                            viewport.clone(),
                            identity_id.clone(),
                            workspace,
                            online,
                        );
                        let events = runtime::start(
                            store.storage.clone(),
                            network_state.clone(),
                            online_state.clone(),
                            identity_id.clone(),
                            account.name,
                            account.bio,
                            None,
                        );
                        runtime_bridge::spawn(
                            events,
                            store.clone(),
                            network_state,
                            online_state,
                            viewport.clone(),
                            chat_state.clone(),
                            identity_id,
                        );
                    }
                    Ok(None) => log::warn!("账户不存在：{account_id}"),
                    Err(err) => log::error!("读取账户失败：{err:?}"),
                }
            });
        }
    });
    let accounts = match store.store.accounts().await {
        Ok(accounts) => accounts,
        Err(err) => {
            log::error!("读取账户失败：{err:?}");
            Vec::new()
        }
    };
    viewport.set_accounts(accounts_model(&accounts));
    virtual_list_layout::setup(&viewport);
    chat::setup(
        &viewport,
        &store,
        &chat_state,
        &online_state,
        &network_state,
    );
    contributors::start(viewport.as_weak());
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    {
        use slint::RenderingState;

        let mut first_frame = true;
        viewport.window().set_rendering_notifier(move |state, _| {
            if !first_frame || !matches!(state, RenderingState::AfterRendering) {
                return;
            }
            first_frame = false;
            ready_notify::notify_ready();
        })?;
    }
    Ok(viewport)
}

fn set_accounts(viewport: slint::Weak<Viewport>, accounts: Vec<runtime::StoreAccount>) {
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            viewport.set_accounts(accounts_model(&accounts));
        }
    });
}

fn set_current_account(viewport: slint::Weak<Viewport>, account: runtime::StoreAccount) {
    let avatar_text = avatar_text(&account.name);
    let avatar = account.avatar;
    let node_id = account
        .endpoint_id
        .as_deref()
        .map(hex::encode)
        .unwrap_or_default();
    _ = slint::invoke_from_event_loop(move || {
        if let Some(viewport) = viewport.upgrade() {
            let image = avatar.as_deref().and_then(image_from_bytes);
            viewport.set_account_avatar_text(avatar_text.into());
            viewport.set_account_has_avatar(image.is_some());
            viewport.set_account_avatar_image(image.unwrap_or_default());
            viewport.set_node_id(node_id.into());
        }
    });
}

fn accounts_model(accounts: &[runtime::StoreAccount]) -> slint::ModelRc<LoginAccount> {
    let accounts = accounts.iter().map(login_account).collect::<Vec<_>>();
    slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(accounts)))
}

fn login_account(account: &runtime::StoreAccount) -> LoginAccount {
    let avatar = account.avatar.as_deref().and_then(image_from_bytes);
    LoginAccount {
        id: account.id.clone().into(),
        name: account.name.clone().into(),
        avatar_text: avatar_text(&account.name).into(),
        has_avatar: avatar.is_some(),
        avatar_image: avatar.unwrap_or_default(),
    }
}

fn parse_endpoint_id(value: &str) -> eyre::Result<[u8; 32]> {
    let value = value.trim();
    let bytes = hex::decode(value).map_err(|err| eyre::eyre!("节点 ID 不是十六进制：{err}"))?;
    bytes
        .try_into()
        .map_err(|_| eyre::eyre!("节点 ID 长度必须为 32 字节"))
}

fn copy_text(text: &str) {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(err) = clipboard.set_text(text.to_string()) {
                    log::warn!("复制节点 ID 失败：{err}");
                }
            }
            Err(err) => log::warn!("打开剪贴板失败：{err}"),
        }
    }
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    {
        let text = text.to_string();
        spawn_task(async move {
            if let Some(window) = web_sys::window() {
                let promise = window.navigator().clipboard().write_text(&text);
                if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
                    log::warn!("复制节点 ID 失败：{err:?}");
                }
            }
        });
    }
    #[cfg(target_os = "android")]
    {
        let _ = text;
    }
}

pub(crate) fn avatar_text(name: &str) -> String {
    name.chars()
        .next()
        .map(|character| character.to_string())
        .unwrap_or_default()
}

pub(crate) fn image_from_bytes(bytes: &[u8]) -> Option<slint::Image> {
    let image = image::load_from_memory(bytes).ok()?.thumbnail(256, 256);
    let image = image.to_rgba8();
    let buffer =
        slint::SharedPixelBuffer::clone_from_slice(image.as_raw(), image.width(), image.height());
    Some(slint::Image::from_rgba8(buffer))
}
