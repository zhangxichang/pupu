use slint::Weak;

use crate::{AppStore, Viewport, chat, spawn_task};
use runtime::{RuntimeEvent, SharedOnline};

pub fn spawn(
    mut events: runtime::RuntimeEvents,
    app: AppStore,
    network: runtime::SharedNetwork,
    online: SharedOnline,
    viewport: Weak<Viewport>,
    chat_state: chat::SharedState,
    identity_id: String,
) {
    spawn_task(async move {
        while let Some(event) = events.recv().await {
            match event {
                RuntimeEvent::ContactsChanged => {
                    reload_contacts(&app.store, &identity_id, &viewport, &online).await;
                }
                RuntimeEvent::ConversationChanged { conversation_id } => {
                    reload_conversation(
                        &app.store,
                        &identity_id,
                        &viewport,
                        &chat_state,
                        &conversation_id,
                    )
                    .await;
                }
                RuntimeEvent::MessageStored {
                    conversation_id,
                    group,
                } => {
                    reload_conversation(
                        &app.store,
                        &identity_id,
                        &viewport,
                        &chat_state,
                        &conversation_id,
                    )
                    .await;
                    if group {
                        if chat::is_group_selected(&chat_state, &conversation_id) {
                            let _ = runtime::send_group_receipt(
                                &app.storage,
                                &network,
                                &identity_id,
                                &conversation_id,
                            )
                            .await;
                        }
                    } else if chat::is_private_selected(&chat_state, &conversation_id) {
                        let _ = runtime::send_receipt(
                            &app.storage,
                            &network,
                            &identity_id,
                            &conversation_id,
                        )
                        .await;
                    }
                }
                RuntimeEvent::Error(message) => log::warn!("运行时错误：{message}"),
            }
        }
    });
}

async fn reload_contacts(
    store: &runtime::Store,
    identity_id: &str,
    viewport: &Weak<Viewport>,
    online: &SharedOnline,
) {
    if let Ok(rows) = store.contacts(identity_id).await {
        let online = runtime::online_snapshot(online);
        chat::apply_contacts(viewport.clone(), rows, &online);
    }
}

async fn reload_conversation(
    store: &runtime::Store,
    identity_id: &str,
    viewport: &Weak<Viewport>,
    chat_state: &chat::SharedState,
    conversation_id: &str,
) {
    let selected = Some(conversation_id.to_string());
    let _ = chat::reload_private(store, identity_id, viewport, chat_state, selected.clone()).await;
    let _ = chat::reload_group(store, identity_id, viewport, chat_state, selected).await;
}
