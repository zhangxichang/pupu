use crate::{
    api::Api,
    error::Error,
    option_ext::{OptionGet, OptionGetClone},
};

#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_connection_send(
    api: tauri::State<'_, Api>,
    id: usize,
    message: String,
) -> Result<(), Error> {
    let inner = api.endpoint.inner.read().get_clone()?;
    let mut send = inner.connections.get(id).get()?.open_uni().await?;
    send.write_all(message.as_bytes()).await?;
    send.finish()?;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_connection_recv(
    api: tauri::State<'_, Api>,
    id: usize,
) -> Result<Option<String>, Error> {
    let mut inner = api.endpoint.inner.read().get_clone()?;
    if let Ok(mut recv) = inner.connections.get(id).get()?.accept_uni().await {
        if let Ok(message) = recv.read_to_end(usize::MAX).await {
            return Ok(Some(String::from_utf8(message)?));
        }
    }
    inner.connections.remove(id);
    Ok(None)
}
