use person_protocol::Event;

use crate::{
    api::Api,
    error::Error,
    option_ext::{OptionGet, OptionGetClone},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_person_protocol_event_next_as_request_remote_id(
    api: tauri::State<'_, Api>,
) -> Result<String, Error> {
    Ok(
        match api
            .endpoint
            .inner
            .read()
            .get()?
            .person_protocol_event_next
            .lock()
            .get()?
        {
            Event::FriendRequest(friend_request) => friend_request.remote_id().to_string(),
            Event::ChatRequest(chat_request) => chat_request.remote_id().to_string(),
        },
    )
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_person_protocol_event_next_as_request_accept(
    api: tauri::State<'_, Api>,
) -> Result<(), Error> {
    Ok(
        match api
            .endpoint
            .inner
            .read()
            .get()?
            .person_protocol_event_next
            .lock()
            .take()
            .get_move()?
        {
            Event::FriendRequest(friend_request) => friend_request.accept()?,
            _ => return Err("返回值被忽略".to_string().into()),
        },
    )
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_person_protocol_event_next_as_request_reject(
    api: tauri::State<'_, Api>,
) -> Result<(), Error> {
    Ok(
        match api
            .endpoint
            .inner
            .read()
            .get()?
            .person_protocol_event_next
            .lock()
            .take()
            .get_move()?
        {
            Event::FriendRequest(friend_request) => friend_request.reject()?,
            Event::ChatRequest(chat_request) => chat_request.reject()?,
        },
    )
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_person_protocol_event_next_as_chat_request_accept(
    api: tauri::State<'_, Api>,
) -> Result<usize, Error> {
    Ok(
        if let Event::ChatRequest(value) = api
            .endpoint
            .inner
            .read()
            .get()?
            .person_protocol_event_next
            .lock()
            .take()
            .get_move()?
        {
            api.endpoint
                .inner
                .read()
                .get_clone()?
                .connections
                .insert(value.accept()?)
        } else {
            return Err("类型错误".to_string().into());
        },
    )
}
