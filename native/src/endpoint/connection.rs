use crate::{error::Error, state::State};

#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_connection_send(
    state: tauri::State<'_, State>,
    id: usize,
    message: String,
) -> Result<(), Error> {
    let connection = state.endpoint.connections.read()[id].clone();
    let mut send = connection.open_uni().await?;
    send.write_all(message.as_bytes()).await?;
    send.finish()?;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_connection_recv(
    state: tauri::State<'_, State>,
    id: usize,
) -> Result<Option<String>, Error> {
    let connection = state.endpoint.connections.read()[id].clone();
    if let Ok(mut recv) = connection.accept_uni().await {
        if let Ok(message) = recv.read_to_end(usize::MAX).await {
            return Ok(Some(String::from_utf8(message)?));
        }
    }
    state.endpoint.connections.write().remove(id);
    Ok(None)
}
