use crate::{
    error::{Error, OptionGet},
    state::State,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_chat_request_remote_id(
    state: tauri::State<'_, State>,
) -> Result<String, Error> {
    Ok(state
        .endpoint
        .chat_request_next
        .lock()
        .get()?
        .remote_id()
        .to_string())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_chat_request_accept(state: tauri::State<'_, State>) -> Result<usize, Error> {
    Ok(state.endpoint.connections.write().insert(
        state
            .endpoint
            .chat_request_next
            .lock()
            .take()
            .get_move()?
            .accept()?,
    ))
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_chat_request_reject(state: tauri::State<'_, State>) -> Result<(), Error> {
    state
        .endpoint
        .chat_request_next
        .lock()
        .take()
        .get_move()?
        .reject()?;
    Ok(())
}
