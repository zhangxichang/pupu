use crate::{
    error::{Error, OptionGet},
    state::State,
};

#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_friend_request_remote_id(
    state: tauri::State<'_, State>,
) -> Result<String, Error> {
    Ok(state
        .endpoint
        .friend_request_next
        .lock()
        .get()?
        .remote_id()
        .to_string())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_friend_request_accept(state: tauri::State<'_, State>) -> Result<(), Error> {
    state
        .endpoint
        .friend_request_next
        .lock()
        .take()
        .get_move()?
        .accept()?;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_friend_request_reject(state: tauri::State<'_, State>) -> Result<(), Error> {
    state
        .endpoint
        .friend_request_next
        .lock()
        .take()
        .get_move()?
        .reject()?;
    Ok(())
}
